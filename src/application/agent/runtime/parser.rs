use super::{AgentDirective, AgentError, ToolRuntime, Value};

impl ToolRuntime {
    pub fn parse_agent_action(&self, content: &str) -> Result<AgentDirective, AgentError> {
        if let Some(value) = extract_json(content) {
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
