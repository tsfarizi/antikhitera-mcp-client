//! FSM-Driven Agent Runner with State Persistence
//!
//! This module provides a reactive, FSM-controlled agent execution flow
//! with automatic state persistence and resumption capabilities.
//!
//! ## Features
//!
//! - **Pause & Resume**: Automatic state serialization on wait states
//! - **Error Recovery**: Formal retry policy with exponential backoff
//! - **Stateless Execution**: Compatible with Cloud Run and ephemeral environments

use super::directive::AgentDirective;
use super::errors::AgentError;
use super::memory::MemoryProvider;
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use super::runtime::json_retry::MAX_JSON_RETRIES;
use super::state::{AgentState, Event, TerminationReason};
use crate::application::client::{ChatRequest, McpClient};
use crate::application::model_provider::ModelProvider;
use crate::logging::AgentLogger;
use serde_json::{Value, json};
use std::sync::Arc;
#[cfg(feature = "native-transport")]
use sysinfo::System;

/// Maximum retry attempts for transient errors
const MAX_TRANSIENT_RETRIES: u32 = 3;

/// Agent runner with FSM-driven execution and stateless resumption
pub struct FsmAgent<P: ModelProvider> {
    pub(super) client: Arc<McpClient<P>>,
    pub(super) runtime: ToolRuntime,
    pub(super) memory: Arc<dyn MemoryProvider>,
}

impl<P: ModelProvider> FsmAgent<P> {
    /// Create a new FSM agent with memory provider
    pub fn new(client: Arc<McpClient<P>>, memory: Arc<dyn MemoryProvider>) -> Self {
        let tools = client.tools().to_vec();
        let bridge = client.server_bridge();
        Self {
            client,
            runtime: ToolRuntime::new(tools, bridge),
            memory,
        }
    }

    /// Run agent with automatic state persistence
    pub async fn run(
        &self,
        prompt: String,
        mut options: AgentOptions,
    ) -> Result<AgentOutcome, AgentError> {
        let context_id = options
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let log = AgentLogger::new(&context_id);
        log.info(format!(
            "Starting FSM-driven agent execution | context_id={}",
            context_id
        ));

        // Start fresh
        log.info(format!(
            "Starting new agent execution | context_id={}",
            context_id
        ));
        let system_prompt = options.system_prompt.take();
        let init_options = AgentOptions {
            system_prompt,
            ..options.clone()
        };
        let state = self.initialize_state(context_id.clone(), init_options)?;

        // Execute FSM loop
        let outcome = self.execute_fsm_loop(state, prompt, options).await?;

        // Save final state
        self.save_state(&outcome).await?;

        Ok(outcome)
    }

    /// Initialize fresh agent state
    fn initialize_state(
        &self,
        _context_id: String,
        options: AgentOptions,
    ) -> Result<AgentState, AgentError> {
        let mut state = AgentState::Idle;

        // Transition to parsing on initial prompt
        state = state.transition(Event::PromptReceived {
            prompt: options.system_prompt.unwrap_or_default(),
        });

        Ok(state)
    }

    /// Execute main FSM loop
    async fn execute_fsm_loop(
        &self,
        initial_state: AgentState,
        prompt: String,
        mut options: AgentOptions,
    ) -> Result<AgentOutcome, AgentError> {
        let mut state = initial_state;
        let mut session_id = options.session_id.clone();
        let log = AgentLogger::new(
            session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );
        let mut steps = Vec::new();
        let mut logs = Vec::new();
        let mut remaining_steps = options.max_steps as u32;
        let mut transient_retries = 0u32;

        // Prepare initial context.  When resuming with an empty prompt the
        // context is built without a user query so that prior conversation
        // history isn't re-framed around a blank message.
        let context = self
            .runtime
            .build_context(if prompt.is_empty() {
                None
            } else {
                Some(&prompt)
            })
            .await;
        let instructions = self
            .runtime
            .compose_system_instructions(&context, self.client.prompts());
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        // Only build an initial user prompt when there is actual input;
        // resumed executions rely on each FSM branch to set next_prompt.
        let mut next_prompt = if prompt.is_empty() {
            String::new()
        } else {
            self.runtime.initial_user_prompt(prompt.clone(), &context)
        };
        if !prompt.is_empty() {
            logs.push(format!(
                "Initial agent request: {}",
                McpClient::<P>::summarise(&prompt)
            ));
        }

        let mut system_prompt_to_send = Some(system_prompt);
        let mut first_call = true;
        let initial_attachments = std::mem::take(&mut options.attachments);
        #[cfg(feature = "native-transport")]
        let mut system = System::new();

        loop {
            // Check for terminal state
            if state.is_terminal() {
                return self.handle_terminal_state(state, session_id, logs, steps);
            }

            // Monitor resources
            #[cfg(feature = "native-transport")]
            {
                system.refresh_cpu_all();
                system.refresh_memory();
                let rss_mb = system.used_memory() / 1024 / 1024;
                let cpu = system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                    / system.cpus().len().max(1) as f32;
                log.debug(format!(
                    "Agent resource utilization | rss_mb={} cpu_usage={} state={}",
                    rss_mb, cpu, state
                ));
            }

            // Handle state-specific logic
            match &state.clone() {
                AgentState::ParsingDirective => {
                    if next_prompt.is_empty() {
                        return Err(AgentError::InvalidResponse(
                            "FSM reached ParsingDirective with an empty prompt; \
                             call resume() with new_input to supply context."
                                .into(),
                        ));
                    }
                    let request = ChatRequest {
                        prompt: next_prompt.clone(),
                        attachments: if first_call {
                            initial_attachments.clone()
                        } else {
                            Vec::new()
                        },
                        system_prompt: if first_call {
                            system_prompt_to_send.take()
                        } else {
                            None
                        },
                        session_id: session_id.clone(),
                        raw_mode: false,
                        bypass_template: true,
                        force_json: true,
                    };

                    match self.client.chat(request).await {
                        Ok(result) => {
                            logs.extend(result.logs.clone());
                            session_id = Some(result.session_id.clone());
                            first_call = false;

                            // Parse directive with retry logic
                            match self
                                .runtime
                                .parse_with_retry(
                                    &result.content,
                                    &self.client,
                                    &mut logs,
                                    &session_id,
                                )
                                .await
                            {
                                Ok(directive) => {
                                    state = self
                                        .handle_directive(
                                            directive,
                                            &mut remaining_steps,
                                            &mut logs,
                                            &mut steps,
                                        )
                                        .await?;
                                }
                                Err(e) => {
                                    state = state.transition(Event::Error {
                                        message: e.to_string(),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            // Transient error - retry with backoff
                            if transient_retries < MAX_TRANSIENT_RETRIES {
                                transient_retries += 1;
                                let delay_ms = 100u64 * (2u64.pow(transient_retries));
                                log.warn(format!(
                                    "Transient error, retrying with exponential backoff | attempt={} delay_ms={}",
                                    transient_retries, delay_ms
                                ));
                                tokio::time::sleep(std::time::Duration::from_millis(delay_ms))
                                    .await;
                                continue;
                            } else {
                                log.error("Max transient retries exceeded");
                                return Err(AgentError::InvalidResponse(e.to_string()));
                            }
                        }
                    }
                }

                AgentState::ExecutingTool { tool_id, input } => {
                    log.info(format!("Executing tool | tool={}", tool_id));

                    match self.runtime.execute(tool_id, input.clone()).await {
                        Ok(execution) => {
                            logs.push(format!(
                                "Tool '{}' executed (success: {})",
                                execution.tool, execution.success
                            ));

                            if let Some(message) = execution.message.as_deref() {
                                logs.push(format!(
                                    "Tool message: {}",
                                    McpClient::<P>::summarise(message)
                                ));
                            }

                            steps.push(AgentStep {
                                tool: execution.tool.clone(),
                                input: execution.input.clone(),
                                success: execution.success,
                                output: execution.output.clone(),
                                message: execution.message.clone(),
                            });

                            // Save state after tool execution (pause point)
                            self.save_intermediate_state(
                                &session_id,
                                &logs,
                                &steps,
                                "ExecutingTool",
                            )
                            .await?;

                            // Prepare tool result prompt
                            let tool_result_instruction =
                                self.client.prompts().tool_result_instruction();
                            next_prompt = json!({
                                "tool_result": {
                                    "tool": execution.tool,
                                    "input": execution.input,
                                    "success": execution.success,
                                    "output": execution.output,
                                    "message": execution.message,
                                },
                                "instruction": tool_result_instruction,
                            })
                            .to_string();

                            state = state.transition(Event::ToolCompleted {
                                tool: execution.tool.clone(),
                                output: execution.output.clone(),
                            });
                        }
                        Err(e) => {
                            state = state.transition(Event::ToolFailed {
                                tool: tool_id.clone(),
                                error: e.to_string(),
                            });
                        }
                    }
                }

                AgentState::WaitingForContext => {
                    // Save state and wait for external input (pause point)
                    log.info("Agent waiting for external context - state persisted");
                    self.save_intermediate_state(&session_id, &logs, &steps, "WaitingForContext")
                        .await?;

                    // In stateless environment, execution would stop here
                    // and resume when new input arrives via resume()
                    return Err(AgentError::InvalidResponse(
                        "Waiting for external context - use resume() to continue".into(),
                    ));
                }

                AgentState::RecoveringError { error, retry_count } => {
                    if *retry_count >= MAX_JSON_RETRIES {
                        log.error(format!("Max error retries exceeded: {}", error));
                        return Err(AgentError::InvalidResponse(error.clone()));
                    }

                    log.info(format!(
                        "Attempting error recovery | retry_count={} error={}",
                        retry_count, error
                    ));

                    // Retry logic would go here
                    state = state.transition(Event::Error {
                        message: error.clone(),
                    });
                }

                AgentState::FinalizingResponse => {
                    state = state.transition(Event::ResponseSent);
                }

                _ => {
                    // Invalid state transition
                    log.warn(format!("Invalid state encountered: {:?}", state));
                    state = AgentState::Terminated {
                        reason: TerminationReason::Error {
                            message: format!("Invalid state: {:?}", state),
                        },
                    };
                }
            }

            // Reset transient retry counter on successful iteration
            transient_retries = 0;
        }
    }

    /// Handle directive from LLM
    async fn handle_directive(
        &self,
        directive: AgentDirective,
        remaining_steps: &mut u32,
        _logs: &mut Vec<String>,
        _steps: &mut Vec<AgentStep>,
    ) -> Result<AgentState, AgentError> {
        let log = AgentLogger::new(&crate::logging::get_active_session());
        match directive {
            AgentDirective::Final { response } => {
                log.info("Agent returned final response");

                // response is already a serde_json::Value — work with it directly
                // rather than round-tripping through a string.
                let (content, data, metadata) = match response {
                    Value::String(s) => (s, None, None),
                    Value::Object(ref obj) => {
                        // Prefer an explicit "response" or "content" text field;
                        // fall back to the full JSON string so nothing is silently
                        // dropped when neither key is present.
                        let content = obj
                            .get("response")
                            .or_else(|| obj.get("content"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                            .unwrap_or_else(|| {
                                serde_json::to_string(&response).unwrap_or_default()
                            });

                        let data = obj.get("data").cloned();

                        let mut metadata_obj = serde_json::Map::new();
                        for (key, value) in obj {
                            if key != "response" && key != "content" && key != "data" {
                                metadata_obj.insert(key.clone(), value.clone());
                            }
                        }
                        let metadata = if metadata_obj.is_empty() {
                            None
                        } else {
                            Some(Value::Object(metadata_obj))
                        };

                        (content, data, metadata)
                    }
                    other => {
                        // Arrays, numbers, booleans, null — serialise to string
                        (other.to_string(), None, None)
                    }
                };

                Ok(AgentState::FinalMessage {
                    content,
                    data,
                    metadata,
                })
            }
            AgentDirective::CallTool { tool, input } => {
                if *remaining_steps == 0 {
                    log.warn("Agent exceeded max tool interactions");
                    return Err(AgentError::MaxStepsExceeded);
                }
                *remaining_steps -= 1;
                log.info(format!("Agent requested tool execution | tool={}", tool));

                Ok(AgentState::ExecutingTool {
                    tool_id: tool,
                    input,
                })
            }
            AgentDirective::CallTools(tools) => {
                if *remaining_steps == 0 {
                    log.warn("Agent exceeded max tool interactions");
                    return Err(AgentError::MaxStepsExceeded);
                }
                *remaining_steps -= 1;

                // The FSM state machine models one active tool execution at a
                // time via `ExecutingTool { tool_id, input }`.  True parallel
                // execution is supported by the non-FSM `Agent` runner which
                // uses `ToolRuntime::execute_parallel`.  Here we execute the
                // first requested tool sequentially; remaining tools are
                // dropped with a warning so the caller is aware.
                if tools.len() > 1 {
                    log.warn(format!(
                        "FsmAgent received CallTools with multiple tools; \
                         only the first will be executed — use Agent for \
                         parallel tool execution | total={}",
                        tools.len()
                    ));
                }
                log.info(format!(
                    "Agent requested tool execution (FSM: sequential) | count={}",
                    tools.len()
                ));

                if let Some((tool, input)) = tools.into_iter().next() {
                    Ok(AgentState::ExecutingTool {
                        tool_id: tool,
                        input,
                    })
                } else {
                    Ok(AgentState::WaitingForContext)
                }
            }
        }
    }
}
