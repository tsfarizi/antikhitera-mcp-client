use super::{ToolError, ToolInvokeError, ToolRuntime, Value};
use tracing::{debug, info, warn};

pub(crate) struct ToolExecution {
    pub tool: String,
    pub success: bool,
    pub input: Value,
    pub output: Value,
    pub message: Option<String>,
}

impl ToolRuntime {
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
