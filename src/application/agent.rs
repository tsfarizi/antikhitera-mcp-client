use crate::client::{ChatRequest, McpClient, McpError};
use crate::config::ToolConfig;
use crate::model::ModelProvider;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};
use utoipa::ToSchema;

const DEFAULT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgentStep {
    pub tool: String,
    #[schema(value_type = Object)]
    pub input: Value,
    pub success: bool,
    #[schema(value_type = Object)]
    pub output: Value,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub session_id: String,
    pub response: String,
    pub steps: Vec<AgentStep>,
}

#[derive(Debug, Clone)]
pub struct AgentOptions {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    pub max_steps: usize,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            model: None,
            system_prompt: None,
            session_id: None,
            max_steps: DEFAULT_MAX_STEPS,
        }
    }
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error(transparent)]
    Client(#[from] McpError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error("invalid agent response: {0}")]
    InvalidResponse(String),
}

impl AgentError {
    pub fn user_message(&self) -> String {
        match self {
            AgentError::Client(err) => err.user_message(),
            AgentError::Tool(err) => err.user_message(),
            AgentError::InvalidResponse(_) => {
                "AI memberikan respons yang tidak dapat dipahami. Coba ulangi instruksi Anda."
                    .to_string()
            }
        }
    }
}

pub struct Agent<P: ModelProvider> {
    client: Arc<McpClient<P>>,
    runtime: ToolRuntime,
}

impl<P: ModelProvider> Agent<P> {
    pub fn new(client: Arc<McpClient<P>>) -> Self {
        let tools = client.tools().to_vec();
        Self {
            client,
            runtime: ToolRuntime::new(tools),
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
        let instructions = self.runtime.protocol_instruction();
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        let mut next_prompt = self.runtime.initial_user_prompt(prompt);
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
                    let execution = self.runtime.execute(&tool, input)?;

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

#[derive(Debug, Clone)]
struct ToolExecution {
    tool: String,
    success: bool,
    input: Value,
    output: Value,
    message: Option<String>,
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("unknown tool requested: {0}")]
    UnknownTool(String),
}

impl ToolError {
    pub fn user_message(&self) -> String {
        match self {
            ToolError::UnknownTool(name) => {
                format!("Tool \"{name}\" belum tersedia di server.")
            }
        }
    }
}

struct ToolRuntime {
    configs: Vec<ToolConfig>,
    index: HashMap<String, ToolConfig>,
}

impl ToolRuntime {
    fn new(configs: Vec<ToolConfig>) -> Self {
        let index = configs
            .iter()
            .cloned()
            .map(|cfg| (cfg.name.to_lowercase(), cfg))
            .collect();
        Self { configs, index }
    }

    fn protocol_instruction(&self) -> String {
        let mut lines = vec![
            "You are an autonomous assistant that can call tools to solve user requests.".to_string(),
            "All responses must be valid JSON without commentary or code fences.".to_string(),
            "When you need to invoke a tool, respond with: {\"action\":\"call_tool\",\"tool\":\"tool_name\",\"input\":{...}}.".to_string(),
            "To obtain the list of available tools, call the special tool: {\"action\":\"call_tool\",\"tool\":\"list_tools\"}.".to_string(),
            "When you are ready to give the final answer to the user, respond with: {\"action\":\"final\",\"response\":\"...\"}.".to_string(),
            "Detect the user's language automatically and answer using that same language unless they explicitly request another language.".to_string(),
            "Do not call any translation-related tools; handle language understanding internally.".to_string(),
        ];

        if self.configs.is_empty() {
            lines.push("No additional tools are currently configured.".to_string());
        } else {
            lines.push("Configured tools:".to_string());
            for tool in &self.configs {
                let description = tool
                    .description
                    .as_deref()
                    .unwrap_or("No description provided.");
                lines.push(format!("- {}: {}", tool.name, description));
            }
        }

        lines.join(" ")
    }

    fn initial_user_prompt(&self, prompt: String) -> String {
        format!(
            "{{\"action\":\"user_request\",\"prompt\":{}}}",
            serde_json::to_string(&prompt).unwrap_or_else(|_| "\"\"".to_string())
        )
    }

    fn parse_agent_action(&self, content: &str) -> Result<AgentDirective, AgentError> {
        if let Some(value) = Self::extract_json(content) {
            return self.parse_action_value(value);
        }
        Ok(AgentDirective::Final {
            response: content.trim().to_string(),
        })
    }

    fn parse_action_value(&self, value: Value) -> Result<AgentDirective, AgentError> {
        if let Some(action) = value.get("action").and_then(Value::as_str) {
            match action {
                "call_tool" => {
                    let tool_name = value
                        .get("tool")
                        .or_else(|| value.get("tool_name"))
                        .or_else(|| value.get("name"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            AgentError::InvalidResponse(
                                "tool name missing in call_tool action".into(),
                            )
                        })?;

                    let input = value
                        .get("input")
                        .or_else(|| value.get("arguments"))
                        .cloned()
                        .unwrap_or(Value::Null);

                    Ok(AgentDirective::CallTool {
                        tool: tool_name.to_string(),
                        input,
                    })
                }
                "final" => {
                    let response = value
                        .get("response")
                        .or_else(|| value.get("answer"))
                        .or_else(|| value.get("content"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            AgentError::InvalidResponse(
                                "final action missing response field".into(),
                            )
                        })?;

                    Ok(AgentDirective::Final {
                        response: response.to_string(),
                    })
                }
                _ => Err(AgentError::InvalidResponse(format!(
                    "unknown action value: {action}"
                ))),
            }
        } else {
            Err(AgentError::InvalidResponse(
                "missing action field in agent response".into(),
            ))
        }
    }

    fn execute(&self, tool_name: &str, input: Value) -> Result<ToolExecution, ToolError> {
        if tool_name.eq_ignore_ascii_case("list_tools") {
            let tools: Vec<Value> = self
                .configs
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                    })
                })
                .collect();
            debug!("Agent requested tool catalogue via list_tools");
            return Ok(ToolExecution {
                tool: "list_tools".to_string(),
                success: true,
                input,
                output: Value::Array(tools),
                message: Some("Configured tools listed successfully.".to_string()),
            });
        }

        if tool_name.eq_ignore_ascii_case("translation") {
            return Ok(ToolExecution {
                tool: "translation".to_string(),
                success: false,
                input,
                output: Value::Null,
                message: Some(
                    "Tidak perlu tool terjemahan. Jawab langsung dalam bahasa yang sama dengan pengguna."
                        .to_string(),
                ),
            });
        }

        let key = tool_name.to_lowercase();
        if let Some(tool) = self.index.get(&key) {
            warn!(
                tool = %tool.name,
                "Tool execution requested but not implemented"
            );
            Ok(ToolExecution {
                tool: tool.name.clone(),
                success: false,
                input,
                output: Value::Null,
                message: Some("Tool execution is not yet implemented.".to_string()),
            })
        } else {
            warn!(requested_tool = %tool_name, "Unknown tool requested by agent");
            Err(ToolError::UnknownTool(tool_name.to_string()))
        }
    }

    fn extract_json(content: &str) -> Option<Value> {
        let trimmed = content.trim();

        // Attempt direct JSON parse
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Some(value);
        }

        // Attempt to parse from code fence
        if trimmed.starts_with("```") {
            let stripped = trimmed.trim_start_matches("```json");
            let stripped = stripped.trim_start_matches("```JSON");
            let stripped = stripped.trim_start_matches("```");
            if let Some(end) = stripped.rfind("```") {
                let slice = &stripped[..end];
                if let Ok(value) = serde_json::from_str::<Value>(slice.trim()) {
                    return Some(value);
                }
            }
        }

        // Attempt to parse substring between braces
        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if start < end {
                let candidate = &trimmed[start..=end];
                if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                    return Some(value);
                }
            }
        }

        None
    }
}

#[derive(Debug)]
enum AgentDirective {
    Final { response: String },
    CallTool { tool: String, input: Value },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientConfig;
    use crate::model::{ModelError, ModelRequest, ModelResponse};
    use crate::types::{ChatMessage, MessageRole};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct ScriptedProvider {
        responses: Arc<Mutex<Vec<String>>>,
        recordings: Arc<Mutex<Vec<ModelRequest>>>,
    }

    impl ScriptedProvider {
        fn new(responses: Vec<&str>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(
                    responses.into_iter().map(String::from).collect(),
                )),
                recordings: Arc::new(Mutex::new(Vec::new())),
            }
        }

        async fn requests(&self) -> Vec<ModelRequest> {
            self.recordings.lock().await.clone()
        }
    }

    #[async_trait]
    impl ModelProvider for ScriptedProvider {
        async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
            let mut responses = self.responses.lock().await;
            let response = responses.remove(0);
            let mut recordings = self.recordings.lock().await;
            recordings.push(request.clone());
            Ok(ModelResponse {
                message: ChatMessage::new(MessageRole::Assistant, response),
                session_id: request.session_id,
            })
        }
    }

    #[tokio::test]
    async fn agent_returns_final_response_without_tools() {
        let provider = ScriptedProvider::new(vec![r#"{"action":"final","response":"done"}"#]);
        let client = McpClient::new(provider.clone(), ClientConfig::new("llama"));
        let agent = Agent::new(Arc::new(client));

        let outcome = agent
            .run("hello world".into(), AgentOptions::default())
            .await
            .expect("agent succeeds");

        assert_eq!(outcome.response, "done");
        assert!(outcome.steps.is_empty());

        let records = provider.requests().await;
        assert!(!records.is_empty());
        assert!(
            records[0]
                .messages
                .iter()
                .any(|msg| msg.content.contains("hello world"))
        );
    }

    #[tokio::test]
    async fn agent_handles_list_tools() {
        let provider = ScriptedProvider::new(vec![
            r#"{"action":"call_tool","tool":"list_tools"}"#,
            r#"{"action":"final","response":"all done"}"#,
        ]);
        let mut cfg = ClientConfig::new("llama");
        cfg = cfg.with_tools(vec![
            ToolConfig {
                name: "weather".into(),
                description: Some("Fetch weather.".into()),
            },
            ToolConfig {
                name: "search".into(),
                description: None,
            },
        ]);
        let client = McpClient::new(provider.clone(), cfg);
        let agent = Agent::new(Arc::new(client));

        let outcome = agent
            .run("need info".into(), AgentOptions::default())
            .await
            .expect("agent succeeds");

        assert_eq!(outcome.response, "all done");
        assert_eq!(outcome.steps.len(), 1);
        assert_eq!(outcome.steps[0].tool, "list_tools");
        assert!(outcome.steps[0].success);

        let records = provider.requests().await;
        assert_eq!(records.len(), 2);
        assert!(
            records[0]
                .messages
                .iter()
                .any(|msg| msg.content.contains("user_request"))
        );
        assert!(
            records[1]
                .messages
                .iter()
                .any(|msg| msg.content.contains("tool_result"))
        );
    }
}
