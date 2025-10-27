use super::directive::AgentDirective;
use super::errors::AgentError;
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use crate::application::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info, warn};

const TOOL_RESULT_GUIDANCE: &str = "Berikan respons JSON valid sesuai format instruksi: gunakan {\"action\":\"call_tool\",\"tool\":\"...\",\"input\":{...}} untuk pemanggilan berikutnya atau {\"action\":\"final\",\"response\":\"...\"} untuk jawaban akhir. Jangan sertakan teks lain di luar struktur JSON.";

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
        logs.push(format!("Permintaan awal agent: {prompt_preview}"));
        let effective_provider = provider_override
            .clone()
            .unwrap_or_else(|| self.client.default_provider().to_string());
        let effective_model = model_override
            .clone()
            .unwrap_or_else(|| self.client.default_model().to_string());
        logs.push(format!(
            "Provider aktif: '{effective_provider}' | Model: '{effective_model}'"
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

            match self.runtime.parse_agent_action(&result.content)? {
                AgentDirective::Final { response } => {
                    info!(
                        session_id = result.session_id.as_str(),
                        "Agent returned final response"
                    );
                    let final_preview = McpClient::<P>::summarise(&response);
                    logs.push(format!("Jawaban akhir agent: {final_preview}"));
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
                        "Tool '{}' dijalankan (sukses: {})",
                        execution.tool, execution.success
                    ));
                    if let Some(message) = execution.message.as_deref() {
                        logs.push(format!(
                            "Pesan tool: {}",
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

                    next_prompt = json!({
                        "tool_result": {
                            "tool": execution.tool,
                            "input": execution.input,
                            "success": execution.success,
                            "output": execution.output,
                            "message": execution.message,
                        },
                        "instruction": TOOL_RESULT_GUIDANCE,
                    })
                    .to_string();
                }
            }
        }
    }
}
