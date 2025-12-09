use super::directive::AgentDirective;
use super::errors::AgentError;
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use crate::application::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Maximum retry attempts for JSON parsing failures
const MAX_JSON_RETRIES: u8 = 3;

pub struct Agent<P: ModelProvider> {
    client: Arc<McpClient<P>>,
    runtime: ToolRuntime,
}

impl<P: ModelProvider> Agent<P> {
    pub fn new(client: Arc<McpClient<P>>) -> Self {
        let tools = client.tools().to_vec();
        let bridge = client.server_bridge();
        Self {
            client,
            runtime: ToolRuntime::new(tools, bridge),
        }
    }

    pub async fn run(
        &self,
        prompt: String,
        mut options: AgentOptions,
    ) -> Result<AgentOutcome, AgentError> {
        info!("Agent run started");
        let mut session_id = options.session_id.clone();
        let mut steps = Vec::new();
        let mut logs = Vec::new();
        let model_override = options.model.clone();
        let provider_override = options.provider.clone();

        let context = self.runtime.build_context().await;
        let instructions = self.runtime.compose_system_instructions(&context);
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        let prompt_preview = McpClient::<P>::summarise(&prompt);
        let mut next_prompt = self.runtime.initial_user_prompt(prompt, &context);
        logs.push(format!("Initial agent request: {prompt_preview}"));
        let effective_provider = provider_override
            .clone()
            .unwrap_or_else(|| self.client.default_provider().to_string());
        let effective_model = model_override
            .clone()
            .unwrap_or_else(|| self.client.default_model().to_string());
        logs.push(format!(
            "Active provider: '{effective_provider}' | Model: '{effective_model}'"
        ));
        let mut remaining_steps = options.max_steps;
        let mut system_prompt_to_send = Some(system_prompt);
        let mut first_call = true;

        loop {
            debug!(
                session = session_id.as_deref(),
                remaining_steps, "Submitting agent turn to model provider"
            );
            let request = ChatRequest {
                prompt: next_prompt.clone(),
                provider: provider_override.clone(),
                model: model_override.clone(),
                system_prompt: if first_call {
                    system_prompt_to_send.take()
                } else {
                    None
                },
                session_id: session_id.clone(),
            };

            let result = self.client.chat(request).await?;
            logs.extend(result.logs.clone());
            session_id = Some(result.session_id.clone());
            first_call = false;

            // Parse agent action with retry logic for malformed JSON
            let directive = self
                .parse_with_retry(&result.content, &mut logs, &session_id)
                .await?;

            match directive {
                AgentDirective::Final { response } => {
                    info!(
                        session_id = result.session_id.as_str(),
                        "Agent returned final response"
                    );
                    let final_preview = McpClient::<P>::summarise(&response);
                    logs.push(format!("Agent final answer: {final_preview}"));
                    return Ok(AgentOutcome {
                        logs,
                        session_id: result.session_id,
                        response,
                        steps,
                    });
                }
                AgentDirective::CallTool { tool, input } => {
                    if remaining_steps == 0 {
                        warn!("Agent exceeded max tool interactions");
                        return Err(AgentError::InvalidResponse(
                            "agent exceeded the maximum number of tool interactions".into(),
                        ));
                    }
                    remaining_steps -= 1;
                    info!(tool = %tool, "Agent requested tool execution");
                    let execution = self.runtime.execute(&tool, input).await?;
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

                    // Use configurable tool result instruction
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
                }
            }
        }
    }

    /// Parse agent action with retry logic for malformed JSON
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
                        "JSON parse failed, requesting correction from model"
                    );
                    logs.push(format!(
                        "JSON parse retry attempt {}/{}: {}",
                        retry_count, MAX_JSON_RETRIES, e
                    ));

                    // Get retry message from config
                    let retry_message = format!(
                        "{}\n\nError details: {}",
                        self.client.prompts().json_retry_message(),
                        e
                    );

                    // Send correction request to model
                    let retry_request = ChatRequest {
                        prompt: retry_message,
                        provider: None,
                        model: None,
                        system_prompt: None,
                        session_id: session_id.clone(),
                    };

                    match self.client.chat(retry_request).await {
                        Ok(retry_result) => {
                            logs.extend(retry_result.logs.clone());
                            current_content = retry_result.content;
                        }
                        Err(chat_err) => {
                            warn!(error = %chat_err, "Retry chat request failed");
                            return Err(AgentError::InvalidResponse(format!(
                                "Failed to get correction after JSON parse error: {}",
                                chat_err
                            )));
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        attempts = retry_count,
                        "JSON parse failed after max retries"
                    );
                    return Err(AgentError::InvalidResponse(format!(
                        "Invalid JSON after {} retry attempts: {}",
                        MAX_JSON_RETRIES, e
                    )));
                }
            }
        }
    }
}
