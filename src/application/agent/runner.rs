use super::directive::AgentDirective;
use super::errors::AgentError;
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use crate::application::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info, warn};

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
        let model_override = options.model.clone();

        let context = self.runtime.build_context().await;
        let instructions = self.runtime.compose_system_instructions(&context);
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        let mut next_prompt = self.runtime.initial_user_prompt(prompt, &context);
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
                model: model_override.clone(),
                system_prompt: if first_call {
                    system_prompt_to_send.take()
                } else {
                    None
                },
                session_id: session_id.clone(),
            };

            let result = self.client.chat(request).await?;
            session_id = Some(result.session_id.clone());
            first_call = false;

            match self.runtime.parse_agent_action(&result.content)? {
                AgentDirective::Final { response } => {
                    info!(
                        session_id = result.session_id.as_str(),
                        "Agent returned final response"
                    );
                    return Ok(AgentOutcome {
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

                    steps.push(AgentStep {
                        tool: execution.tool.clone(),
                        input: execution.input.clone(),
                        success: execution.success,
                        output: execution.output.clone(),
                        message: execution.message.clone(),
                    });

                    next_prompt = json!({
                        "tool_result": {
                            "tool": execution.tool,
                            "input": execution.input,
                            "success": execution.success,
                            "output": execution.output,
                            "message": execution.message,
                        }
                    })
                    .to_string();
                }
            }
        }
    }
}
