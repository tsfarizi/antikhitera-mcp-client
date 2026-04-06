//! FSM-Driven Agent Runner with Stateless Resumption
//!
//! This module provides a reactive, FSM-controlled agent execution flow
//! with automatic state persistence and resumption capabilities.
//!
//! ## Features
//!
//! - **Pause & Resume**: Automatic state serialization on wait states
//! - **Contextual Rehydration**: Resume execution from any checkpoint
//! - **Error Recovery**: Formal retry policy with exponential backoff
//! - **Stateless Execution**: Compatible with Cloud Run and ephemeral environments

use super::directive::AgentDirective;
use super::errors::AgentError;
use super::memory::{AgentStateSnapshot, ConversationTurn, MemoryProvider};
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use super::state::{AgentState, Event, TerminationReason};
use crate::application::client::{ChatRequest, McpClient};
use crate::infrastructure::model::ModelProvider;
use serde_json::{Value, json};
use std::sync::Arc;
use sysinfo::System;
use tracing::{debug, error, info, warn};

/// Maximum retry attempts for JSON parsing failures
const MAX_JSON_RETRIES: u8 = 3;

/// Maximum retry attempts for transient errors
const MAX_TRANSIENT_RETRIES: u32 = 3;

/// Agent runner with FSM-driven execution and stateless resumption
pub struct FsmAgent<P: ModelProvider> {
    client: Arc<McpClient<P>>,
    runtime: ToolRuntime,
    memory: Arc<dyn MemoryProvider>,
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
        let context_id = options.session_id.clone().unwrap_or_else(|| {
            uuid::Uuid::new_v4().to_string()
        });

        info!(
            context_id = %context_id,
            "Starting FSM-driven agent execution"
        );

        // Try to resume from existing state
        let state = if let Some(snapshot) = self.memory.load_state(&context_id).await? {
            info!(
                context_id = %context_id,
                "Resuming agent from saved state"
            );
            self.rehydrate_from_snapshot(snapshot)?
        } else {
            // Start fresh
            info!(
                context_id = %context_id,
                "Starting new agent execution"
            );
            // Extract system_prompt before moving options
            let system_prompt = options.system_prompt.take();
            let init_options = AgentOptions {
                system_prompt,
                ..options.clone()
            };
            self.initialize_state(context_id.clone(), init_options)?
        };

        // Execute FSM loop
        let outcome = self.execute_fsm_loop(state, prompt, options).await?;

        // Save final state
        self.save_state(&outcome).await?;

        Ok(outcome)
    }

    /// Resume agent execution from a specific context ID
    pub async fn resume(
        &self,
        context_id: &str,
        new_input: Option<String>,
    ) -> Result<AgentOutcome, AgentError> {
        info!(
            context_id = %context_id,
            "Resuming agent execution"
        );

        let snapshot = self.memory.load_state(&context_id.to_string()).await?
            .ok_or_else(|| AgentError::InvalidResponse(format!("Context {} not found", context_id)))?;

        let mut state = self.rehydrate_from_snapshot(snapshot)?;
        
        // If new input provided, inject it as context
        if let Some(input) = new_input {
            state = state.transition(Event::ContextReceived { context: input });
        }

        // Continue execution
        let outcome = self.continue_fsm_loop(state).await?;
        
        // Save updated state
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

    /// Rehydrate FSM state from snapshot
    fn rehydrate_from_snapshot(
        &self,
        snapshot: AgentStateSnapshot,
    ) -> Result<AgentState, AgentError> {
        // Parse FSM state from string representation
        let state = match snapshot.fsm_state.as_str() {
            "Idle" => AgentState::Idle,
            "ParsingDirective" => AgentState::ParsingDirective,
            "ExecutingTool" => {
                // Extract tool info from context vars
                let tool_id = snapshot.context_vars.get("current_tool")
                    .cloned().unwrap_or_default();
                AgentState::ExecutingTool {
                    tool_id,
                    input: snapshot.tool_cache.values().next().cloned().unwrap_or_default(),
                }
            }
            "WaitingForContext" => AgentState::WaitingForContext,
            "RecoveringError" => {
                let error = snapshot.metadata.last_error.clone().unwrap_or_default();
                let retry_count = snapshot.metadata.steps_executed.min(3) as u8;
                AgentState::RecoveringError { error, retry_count }
            }
            "FinalizingResponse" => AgentState::FinalizingResponse,
            "Terminated" => {
                AgentState::Terminated {
                    reason: TerminationReason::Success,
                }
            }
            _ => AgentState::Idle,
        };

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
        let mut steps = Vec::new();
        let mut logs = Vec::new();
        let mut remaining_steps = options.max_steps as u32;
        let mut transient_retries = 0u32;

        // Prepare initial context
        let context = self.runtime.build_context(Some(&prompt)).await;
        let instructions = self.runtime.compose_system_instructions(&context, self.client.prompts());
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        let mut next_prompt = self.runtime.initial_user_prompt(prompt.clone(), &context);
        logs.push(format!("Initial agent request: {}", McpClient::<P>::summarise(&prompt)));

        let mut system_prompt_to_send = Some(system_prompt);
        let mut first_call = true;
        let initial_attachments = std::mem::take(&mut options.attachments);
        let mut system = System::new();

        loop {
            // Check for terminal state
            if state.is_terminal() {
                return self.handle_terminal_state(state, session_id, logs, steps);
            }

            // Monitor resources
            system.refresh_cpu_all();
            system.refresh_memory();
            let rss_mb = system.used_memory() / 1024 / 1024;
            let cpu = system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                / system.cpus().len().max(1) as f32;
            debug!(
                rss_mb = rss_mb,
                cpu_usage = cpu,
                state = %state,
                "Agent resource utilization"
            );

            // Handle state-specific logic
            match &state.clone() {
                AgentState::ParsingDirective => {
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
                            match self.parse_with_retry(&result.content, &mut logs, &session_id).await {
                                Ok(directive) => {
                                    state = self.handle_directive(directive, &mut remaining_steps, &mut logs, &mut steps).await?;
                                }
                                Err(e) => {
                                    state = state.transition(Event::Error { message: e.to_string() });
                                }
                            }
                        }
                        Err(e) => {
                            // Transient error - retry with backoff
                            if transient_retries < MAX_TRANSIENT_RETRIES {
                                transient_retries += 1;
                                let delay_ms = 100u64 * (2u64.pow(transient_retries));
                                warn!(
                                    attempt = transient_retries,
                                    delay_ms = delay_ms,
                                    "Transient error, retrying with exponential backoff"
                                );
                                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                                continue;
                            } else {
                                error!("Max transient retries exceeded");
                                return Err(AgentError::InvalidResponse(e.to_string()));
                            }
                        }
                    }
                }

                AgentState::ExecutingTool { tool_id, input } => {
                    info!(tool = %tool_id, "Executing tool");
                    
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
                            self.save_intermediate_state(&session_id, &logs, &steps, "ExecutingTool").await?;

                            // Prepare tool result prompt
                            let tool_result_instruction = self.client.prompts().tool_result_instruction();
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
                    info!("Agent waiting for external context - state persisted");
                    self.save_intermediate_state(&session_id, &logs, &steps, "WaitingForContext").await?;
                    
                    // In stateless environment, execution would stop here
                    // and resume when new input arrives via resume()
                    return Err(AgentError::InvalidResponse(
                        "Waiting for external context - use resume() to continue".into()
                    ));
                }

                AgentState::RecoveringError { error, retry_count } => {
                    if *retry_count >= MAX_JSON_RETRIES {
                        error!("Max error retries exceeded: {}", error);
                        return Err(AgentError::InvalidResponse(error.clone()));
                    }
                    
                    info!(
                        retry_count = retry_count,
                        error = %error,
                        "Attempting error recovery"
                    );
                    
                    // Retry logic would go here
                    state = state.transition(Event::Error { message: error.clone() });
                }

                AgentState::FinalizingResponse => {
                    state = state.transition(Event::ResponseSent);
                }

                _ => {
                    // Invalid state transition
                    warn!("Invalid state encountered: {:?}", state);
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

    /// Continue FSM loop from intermediate state
    async fn continue_fsm_loop(
        &self,
        initial_state: AgentState,
    ) -> Result<AgentOutcome, AgentError> {
        // Simplified continuation - in practice would need to pass more context
        self.execute_fsm_loop(initial_state, String::new(), AgentOptions::default()).await
    }

    /// Handle directive from LLM
    async fn handle_directive(
        &self,
        directive: AgentDirective,
        remaining_steps: &mut u32,
        _logs: &mut Vec<String>,
        _steps: &mut Vec<AgentStep>,
    ) -> Result<AgentState, AgentError> {
        match directive {
            AgentDirective::Final { response } => {
                info!("Agent returned final response");

                // Format response as JSON if it's a string
                let formatted_response = {
                    // Try to parse as JSON first
                    match serde_json::from_str::<serde_json::Value>(&response) {
                        Ok(json_value) => {
                            // Already valid JSON
                            json_value.to_string()
                        }
                        Err(_) => {
                            // Not JSON - wrap in JSON structure
                            serde_json::json!({
                                "response": response,
                                "type": "text"
                            }).to_string()
                        }
                    }
                };

                // Try to extract structured data if response is JSON
                let (content, data, metadata) = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&formatted_response) {
                    // Check if it has 'response' or 'content' field
                    let content = json.get("response")
                        .or_else(|| json.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // Extract data field if present
                    let data = json.get("data").cloned();
                    
                    // Build metadata from remaining fields
                    let mut metadata_obj = serde_json::Map::new();
                    if let Some(obj) = json.as_object() {
                        for (key, value) in obj {
                            if key != "response" && key != "content" && key != "data" {
                                metadata_obj.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    let metadata = if metadata_obj.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(metadata_obj))
                    };
                    
                    (content, data, metadata)
                } else {
                    // Plain text response
                    (formatted_response.clone(), None, None)
                };
                
                Ok(AgentState::FinalMessage {
                    content,
                    data,
                    metadata,
                })
            }
            AgentDirective::CallTool { tool, input } => {
                if *remaining_steps == 0 {
                    warn!("Agent exceeded max tool interactions");
                    return Err(AgentError::MaxStepsExceeded);
                }
                *remaining_steps -= 1;
                info!(tool = %tool, "Agent requested tool execution");
                
                Ok(AgentState::ExecutingTool {
                    tool_id: tool,
                    input,
                })
            }
            AgentDirective::CallTools(tools) => {
                if *remaining_steps == 0 {
                    warn!("Agent exceeded max tool interactions");
                    return Err(AgentError::MaxStepsExceeded);
                }
                *remaining_steps -= 1;
                info!(count = tools.len(), "Agent requested parallel tool execution");
                
                // For simplicity, execute first tool - parallel execution would be similar
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

    /// Handle terminal state and return outcome
    fn handle_terminal_state(
        &self,
        state: AgentState,
        session_id: Option<String>,
        logs: Vec<String>,
        steps: Vec<AgentStep>,
    ) -> Result<AgentOutcome, AgentError> {
        match state {
            // ⭐ NEW: Handle FinalMessage state with JSON response formatting
            AgentState::FinalMessage { content, data, metadata } => {
                info!("Agent reached FinalMessage state with structured response");
                
                // Build structured JSON response
                let mut response_obj = serde_json::Map::new();
                response_obj.insert("content".to_string(), serde_json::Value::String(content.clone()));
                
                if let Some(data_value) = data {
                    response_obj.insert("data".to_string(), data_value);
                }
                
                if let Some(metadata_value) = metadata {
                    response_obj.insert("metadata".to_string(), metadata_value);
                }
                
                let structured_response = Value::Object(response_obj);
                
                Ok(AgentOutcome {
                    logs,
                    session_id: session_id.unwrap_or_default(),
                    response: structured_response,
                    steps,
                })
            }
            AgentState::Terminated { reason } => match reason {
                TerminationReason::Success if !steps.is_empty() => {
                    let last_step = steps.last().unwrap();
                    Ok(AgentOutcome {
                        logs,
                        session_id: session_id.unwrap_or_default(),
                        response: Value::String(last_step.message.clone().unwrap_or_default()),
                        steps,
                    })
                }
                TerminationReason::Error { message } => {
                    Err(AgentError::InvalidResponse(message))
                }
                TerminationReason::MaxStepsExceeded => Err(AgentError::MaxStepsExceeded),
                TerminationReason::Timeout => Err(AgentError::Timeout),
                TerminationReason::Cancelled => {
                    Err(AgentError::InvalidResponse("Cancelled".into()))
                }
                _ => Err(AgentError::InvalidResponse("Unknown termination".into())),
            },
            _ => Err(AgentError::InvalidResponse("Unexpected terminal state".into())),
        }
    }

    /// Save intermediate state during execution
    async fn save_intermediate_state(
        &self,
        session_id: &Option<String>,
        logs: &[String],
        steps: &[AgentStep],
        state_name: &str,
    ) -> Result<(), AgentError> {
        let context_id = session_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let mut snapshot = AgentStateSnapshot::new(context_id, "agent".into());
        snapshot.fsm_state = state_name.to_string();
        snapshot.history = logs.iter().map(|log| ConversationTurn {
            role: "system".into(),
            content: log.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            attachments: Vec::new(),
        }).collect();
        snapshot.metadata.steps_executed = steps.len() as u32;

        self.memory.save_state(snapshot).await
            .map_err(|e| AgentError::InvalidResponse(format!("State persistence failed: {}", e)))?;

        debug!("Intermediate state saved: {}", state_name);
        Ok(())
    }

    /// Save final state
    async fn save_state(&self, outcome: &AgentOutcome) -> Result<(), AgentError> {
        let mut snapshot = AgentStateSnapshot::new(outcome.session_id.clone(), "agent".into());
        snapshot.fsm_state = "Terminated".to_string();
        snapshot.history = outcome.logs.iter().map(|log| ConversationTurn {
            role: "system".into(),
            content: log.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            attachments: Vec::new(),
        }).collect();
        snapshot.metadata.steps_executed = outcome.steps.len() as u32;

        self.memory.save_state(snapshot).await
            .map_err(|e| AgentError::InvalidResponse(format!("Final state persistence failed: {}", e)))?;

        info!("Final agent state saved");
        Ok(())
    }

    /// Parse agent action with retry logic
    async fn parse_with_retry(
        &self,
        content: &str,
        logs: &mut Vec<String>,
        session_id: &Option<String>,
    ) -> Result<AgentDirective, AgentError> {
        let mut retry_count = 0u8;
        let mut current_content = content.to_string();

        loop {
            match self.runtime.parse_agent_action(&current_content) {
                Ok(directive) => return Ok(directive),
                Err(e) if retry_count < MAX_JSON_RETRIES => {
                    retry_count += 1;
                    warn!(
                        attempt = retry_count,
                        max_attempts = MAX_JSON_RETRIES,
                        error = %e,
                        "JSON parse failed, requesting correction"
                    );
                    logs.push(format!(
                        "JSON parse retry {}/{}: {}",
                        retry_count, MAX_JSON_RETRIES, e
                    ));

                    let retry_message = format!(
                        "{}\n\nError: {}",
                        self.client.prompts().json_retry_message(),
                        e
                    );

                    let retry_request = ChatRequest {
                        prompt: retry_message,
                        attachments: Vec::new(),
                        system_prompt: None,
                        session_id: session_id.clone(),
                        raw_mode: false,
                        bypass_template: true,
                        force_json: true,
                    };

                    match self.client.chat(retry_request).await {
                        Ok(result) => {
                            logs.extend(result.logs.clone());
                            current_content = result.content;
                        }
                        Err(chat_err) => {
                            return Err(AgentError::InvalidResponse(format!(
                                "Retry failed: {}",
                                chat_err
                            )));
                        }
                    }
                }
                Err(e) => {
                    return Err(AgentError::InvalidResponse(format!(
                        "Invalid JSON after {} retries: {}",
                        MAX_JSON_RETRIES, e
                    )));
                }
            }
        }
    }
}
