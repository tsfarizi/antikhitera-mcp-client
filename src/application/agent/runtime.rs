use super::context::{ServerGuidance, ToolContext, ToolDescriptor};
use super::directive::AgentDirective;
use super::errors::{AgentError, ToolError};
use crate::application::tooling::{ToolInvokeError, ToolServerInterface};
use crate::config::ToolConfig;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct ToolRuntime {
    configs: Vec<ToolConfig>,
    index: HashMap<String, ToolConfig>,
    bridge: Arc<dyn ToolServerInterface>,
}

impl ToolRuntime {
    pub fn new(configs: Vec<ToolConfig>, bridge: Arc<dyn ToolServerInterface>) -> Self {
        let index = configs
            .iter()
            .cloned()
            .map(|cfg| (cfg.name.to_lowercase(), cfg))
            .collect();
        Self {
            configs,
            index,
            bridge,
        }
    }

    pub async fn build_context(&self) -> ToolContext {
        if self.configs.is_empty() {
            return ToolContext::default();
        }

        let mut context = ToolContext::default();
        let mut seen_servers = HashSet::new();

        for tool in &self.configs {
            if let Some(server_name) = tool.server.as_deref() {
                if seen_servers.insert(server_name.to_string()) {
                    if let Some(instruction) = self.bridge.server_instructions(server_name).await {
                        context.servers.push(ServerGuidance {
                            name: server_name.to_string(),
                            instruction,
                        });
                    }
                }
            }

            let mut descriptor = ToolDescriptor {
                name: tool.name.clone(),
                description: tool.description.clone(),
                server: tool.server.clone(),
                input_schema: None,
            };

            if let Some(server_name) = tool.server.as_deref() {
                if let Some(metadata) = self.bridge.tool_metadata(server_name, &tool.name).await {
                    if !metadata.name.is_empty() {
                        descriptor.name = metadata.name;
                    }
                    if let Some(remote_desc) = metadata.description {
                        descriptor.description = match descriptor.description {
                            Some(existing)
                                if existing.trim().is_empty()
                                    || existing.trim() == remote_desc.trim() =>
                            {
                                Some(remote_desc)
                            }
                            Some(existing) => {
                                Some(format!("{} {}", remote_desc.trim(), existing.trim()))
                            }
                            None => Some(remote_desc),
                        };
                    }
                    descriptor.input_schema = metadata.input_schema;
                }
            }

            context.tools.push(descriptor);
        }

        context
    }

    pub fn compose_system_instructions(&self, context: &ToolContext) -> String {
        let mut lines = vec![
            "You are an autonomous assistant that can call tools to solve user requests."
                .to_string(),
            "All responses must be valid JSON without commentary or code fences.".to_string(),
            "When you need to invoke a tool, respond with: {\"action\":\"call_tool\",\"tool\":\"tool_name\",\"input\":{...}}."
                .to_string(),
            "To obtain the list of available tools, call the special tool: {\"action\":\"call_tool\",\"tool\":\"list_tools\"}."
                .to_string(),
            "When you are ready to give the final answer to the user, respond with: {\"action\":\"final\",\"response\":\"...\"}."
                .to_string(),
            "Detect the user's language automatically and answer using that same language unless they explicitly request another language."
                .to_string(),
            "Do not call any translation-related tools; handle language understanding internally."
                .to_string(),
        ];

        if context.is_empty() {
            lines.push("No additional tools are currently configured.".to_string());
            return lines.join(" ");
        }

        for guidance in &context.servers {
            lines.push(format!(
                "Server '{}' guidance: {}",
                guidance.name, guidance.instruction
            ));
        }

        if !context.tools.is_empty() {
            lines.push("Configured tools:".to_string());
            for descriptor in &context.tools {
                let mut line = format!("- {}", descriptor.name);
                if let Some(server) = &descriptor.server {
                    line.push_str(&format!(" (server: {})", server));
                }
                if let Some(description) = &descriptor.description {
                    line.push_str(&format!(": {}", description));
                }
                if let Some(schema) = &descriptor.input_schema {
                    let compact = serde_json::to_string(schema).unwrap_or_default();
                    line.push_str(&format!(". Input schema: {}", compact));
                }
                lines.push(line);
            }
        }

        lines.join(" ")
    }

    pub fn initial_user_prompt(&self, prompt: String, context: &ToolContext) -> String {
        let mut payload = json!({
            "action": "user_request",
            "prompt": prompt,
        });

        if !context.is_empty() {
            if let Some(map) = payload.as_object_mut() {
                if let Ok(value) = serde_json::to_value(context) {
                    map.insert("tool_context".to_string(), value);
                }
            }
        }

        payload.to_string()
    }

    pub(crate) async fn execute(
        &self,
        tool_name: &str,
        input: Value,
    ) -> Result<ToolExecution, ToolError> {
        if tool_name.eq_ignore_ascii_case("list_tools") {
            let manifest = self.build_context().await;
            let output = serde_json::to_value(&manifest).unwrap_or_else(|_| Value::Null);
            debug!("Agent requested tool catalogue via list_tools");
            let execution = ToolExecution {
                tool: "list_tools".to_string(),
                success: true,
                input,
                output,
                message: Some(format!(
                    "Configured tools tersedia: {} item.",
                    manifest.tools.len()
                )),
            };
            info!(tool = %execution.tool, success = execution.success, "Tool executed");
            return Ok(execution);
        }

        if tool_name.eq_ignore_ascii_case("translation") {
            let execution = ToolExecution {
                tool: "translation".to_string(),
                success: false,
                input,
                output: Value::Null,
                message: Some(
                    "Tidak perlu tool terjemahan. Jawab langsung dalam bahasa yang sama dengan pengguna."
                        .to_string(),
                ),
            };
            info!(tool = %execution.tool, success = execution.success, "Tool executed");
            return Ok(execution);
        }

        let key = tool_name.to_lowercase();
        let Some(tool) = self.index.get(&key).cloned() else {
            warn!(requested_tool = %tool_name, "Unknown tool requested by agent");
            return Err(ToolError::UnknownTool(tool_name.to_string()));
        };

        let tool_name = tool.name.clone();

        let server_name = match tool.server.as_deref() {
            Some(name) => name,
            None => {
                warn!(tool = %tool_name, "Tool configured without server binding");
                return Err(ToolError::UnboundTool(tool_name));
            }
        };

        let arguments = match input.clone() {
            Value::Null => Value::Object(Default::default()),
            other => other,
        };

        debug!(tool = %tool_name, server = %server_name, "Dispatching tool via MCP");
        match self
            .bridge
            .invoke_tool(server_name, &tool_name, arguments)
            .await
        {
            Ok(result) => {
                let is_error = result
                    .get("isError")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let message = extract_tool_message(&result);
                let execution = ToolExecution {
                    tool: tool_name,
                    success: !is_error,
                    input,
                    output: result,
                    message,
                };
                info!(tool = %execution.tool, success = execution.success, "Tool executed");
                Ok(execution)
            }
            Err(ToolInvokeError::NotConfigured { .. }) => Err(ToolError::UnboundTool(tool_name)),
            Err(source) => {
                warn!(tool = %tool_name, server = %server_name, %source, "Tool execution failed");
                Err(ToolError::Execution {
                    tool: tool_name,
                    source,
                })
            }
        }
    }

    pub fn parse_agent_action(&self, content: &str) -> Result<AgentDirective, AgentError> {
        if let Some(value) = Self::extract_json(content) {
            self.parse_action_value(value)
        } else {
            Err(AgentError::InvalidResponse(
                "expected JSON object in agent response".into(),
            ))
        }
    }

    fn parse_action_value(&self, value: Value) -> Result<AgentDirective, AgentError> {
        match value {
            Value::Object(map) => {
                if let Some(action) = map.get("action").and_then(Value::as_str) {
                    match action {
                        "call_tool" => {
                            let tool =
                                map.get("tool").and_then(Value::as_str).ok_or_else(|| {
                                    AgentError::InvalidResponse(
                                        "call_tool action missing tool field".into(),
                                    )
                                })?;
                            let input = map.get("input").cloned().unwrap_or(Value::Null);
                            Ok(AgentDirective::CallTool {
                                tool: tool.to_string(),
                                input,
                            })
                        }
                        "final" => {
                            let response =
                                map.get("response").and_then(Value::as_str).ok_or_else(|| {
                                    AgentError::InvalidResponse(
                                        "final action missing response field".into(),
                                    )
                                })?;

                            Ok(AgentDirective::Final {
                                response: response.to_string(),
                            })
                        }
                        other => Err(AgentError::InvalidResponse(format!(
                            "unknown action value: {other}"
                        ))),
                    }
                } else {
                    Err(AgentError::InvalidResponse(
                        "missing action field in agent response".into(),
                    ))
                }
            }
            Value::String(text) => self.parse_agent_action(&text),
            other => Err(AgentError::InvalidResponse(format!(
                "unsupported response type: {other}"
            ))),
        }
    }

    fn extract_json(content: &str) -> Option<Value> {
        let trimmed = content.trim();

        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Some(value);
        }

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

pub(crate) struct ToolExecution {
    pub tool: String,
    pub success: bool,
    pub input: Value,
    pub output: Value,
    pub message: Option<String>,
}

fn extract_tool_message(result: &Value) -> Option<String> {
    if let Some(array) = result.get("content").and_then(Value::as_array) {
        for block in array {
            if block
                .get("type")
                .and_then(Value::as_str)
                .map(|value| value.eq_ignore_ascii_case("text"))
                .unwrap_or(false)
            {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
    }

    if let Some(structured) = result.get("structuredContent").and_then(Value::as_object) {
        if let Some(error) = structured.get("error").and_then(Value::as_object) {
            if let Some(message) = error.get("message").and_then(Value::as_str) {
                let trimmed = message.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    None
}
