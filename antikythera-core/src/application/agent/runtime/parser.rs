use super::{AgentDirective, AgentError, ToolRuntime, Value};
use std::time::Instant;
use tracing::{debug, warn};

impl ToolRuntime {
    pub fn parse_agent_action(&self, content: &str) -> Result<AgentDirective, AgentError> {
        let start_time = Instant::now();
        let result = if let Some(value) = extract_json(content) {
            self.parse_action_value(value)
        } else {
            Err(AgentError::InvalidResponse(
                "expected JSON object in agent response".into(),
            ))
        };
        let elapsed = start_time.elapsed();
        debug!(latency_us = ?elapsed.as_micros(), "Action parsing completed");
        result
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
                            let response = map.get("response").ok_or_else(|| {
                                AgentError::InvalidResponse(
                                    "final action missing response field".into(),
                                )
                            })?;

                            Ok(AgentDirective::Final {
                                response: response.as_str().unwrap_or("").to_string(),
                            })
                        }
                        other => {
                            // Unknown action: try to extract a response string using
                            // the configurable fallback keys.  Extra fields the model
                            // may add are simply ignored — developers can add proper
                            // variants to AgentDirective when they need to handle a
                            // new action type explicitly.
                            let response = self
                                .fallback_response_keys
                                .iter()
                                .find_map(|k| map.get(k.as_str()).and_then(Value::as_str))
                                .map(str::to_string)
                                .unwrap_or_else(|| {
                                    serde_json::to_string(&Value::Object(map.clone()))
                                        .unwrap_or_default()
                                });
                            warn!(
                                action = other,
                                "Unknown action value — treating as final response"
                            );
                            Ok(AgentDirective::Final { response })
                        }
                    }
                } else {
                    // No `action` field: probe the same configurable fallback keys
                    // before giving up.
                    let response = self
                        .fallback_response_keys
                        .iter()
                        .find_map(|k| map.get(k.as_str()).and_then(Value::as_str))
                        .map(str::to_string);

                    if let Some(r) = response {
                        warn!("No action field in agent response — treating as final response");
                        Ok(AgentDirective::Final { response: r })
                    } else {
                        Err(AgentError::InvalidResponse(
                            "missing action field in agent response".into(),
                        ))
                    }
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

    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        let candidate = &trimmed[start..=end];
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }

    None
}
