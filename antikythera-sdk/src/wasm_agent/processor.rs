//! WASM Agent Processor
//!
//! Processes LLM responses and determines next action.
//! WASM does NOT call LLM APIs - host handles that.

use super::types::*;
use std::collections::HashMap;

// ============================================================================
// LLM Response Processing
// ============================================================================

/// Process LLM response and determine next action.
///
/// This parser supports:
/// - Framework-generic JSON format (`action`, `tool`, `input`)
/// - OpenAI native tool-calling shape
/// - Gemini native function-call shape
/// - Anthropic native tool_use shape
/// - Plain-text fallback
pub fn process_llm_response(
    state: &mut AgentState,
    llm_response_content: &str,
) -> Result<AgentAction, String> {
    if state.is_max_steps_exceeded() {
        return Err(format!(
            "Max steps exceeded: {} >= {}",
            state.current_step, state.config.max_steps
        ));
    }

    let parsed: serde_json::Value = match serde_json::from_str(llm_response_content) {
        Ok(value) => value,
        Err(_) => {
            return Ok(AgentAction::Final {
                response: serde_json::Value::String(llm_response_content.to_string()),
            })
        }
    };

    if let Some(tool_action) = parse_generic_tool_action(&parsed)? {
        state.current_step += 1;
        return Ok(tool_action);
    }

    if let Some(tool_action) = parse_openai_tool_action(&parsed) {
        state.current_step += 1;
        return Ok(tool_action);
    }

    if let Some(tool_action) = parse_gemini_tool_action(&parsed) {
        state.current_step += 1;
        return Ok(tool_action);
    }

    if let Some(tool_action) = parse_anthropic_tool_action(&parsed) {
        state.current_step += 1;
        return Ok(tool_action);
    }

    if let Some(final_response) = parse_final_response(&parsed) {
        return Ok(AgentAction::Final {
            response: final_response,
        });
    }

    Err("Could not derive action from LLM response".to_string())
}

fn parse_generic_tool_action(parsed: &serde_json::Value) -> Result<Option<AgentAction>, String> {
    let action = match parsed.get("action").and_then(|v| v.as_str()) {
        Some(action) => action,
        None => return Ok(None),
    };

    match action {
        "call_tool" => {
            let tool = parsed
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'tool' field")?
                .to_string();

            let input = parsed
                .get("input")
                .or_else(|| parsed.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            Ok(Some(AgentAction::CallTool { tool, input }))
        }
        "final" => {
            let response = parsed
                .get("response")
                .or_else(|| parsed.get("content"))
                .cloned()
                .ok_or("Missing 'response' field")?;
            Ok(Some(AgentAction::Final { response }))
        }
        "retry" => {
            let error = parsed
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("retry requested")
                .to_string();
            Ok(Some(AgentAction::Retry { error }))
        }
        _ => Err(format!("Unknown action: {}", action)),
    }
}

fn parse_openai_tool_action(parsed: &serde_json::Value) -> Option<AgentAction> {
    let message = parsed
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?;

    let tool_call = message.get("tool_calls")?.as_array()?.first()?;
    let function = tool_call.get("function")?;
    let tool = function.get("name")?.as_str()?.to_string();
    let arguments_raw = function.get("arguments")?.as_str().unwrap_or("{}");

    let input = serde_json::from_str(arguments_raw).unwrap_or_else(|_| serde_json::json!({}));
    Some(AgentAction::CallTool { tool, input })
}

fn parse_gemini_tool_action(parsed: &serde_json::Value) -> Option<AgentAction> {
    let parts = parsed
        .get("candidates")?
        .as_array()?
        .first()?
        .get("content")?
        .get("parts")?
        .as_array()?;

    for part in parts {
        if let Some(function_call) = part.get("functionCall") {
            let tool = function_call.get("name")?.as_str()?.to_string();
            let input = function_call
                .get("args")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            return Some(AgentAction::CallTool { tool, input });
        }
    }

    None
}

fn parse_anthropic_tool_action(parsed: &serde_json::Value) -> Option<AgentAction> {
    let content = parsed.get("content")?.as_array()?;
    for entry in content {
        if entry.get("type")?.as_str()? == "tool_use" {
            let tool = entry.get("name")?.as_str()?.to_string();
            let input = entry
                .get("input")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            return Some(AgentAction::CallTool { tool, input });
        }
    }

    None
}

fn parse_final_response(parsed: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(response) = parsed.get("response").or_else(|| parsed.get("content")) {
        return Some(response.clone());
    }

    let openai_content = parsed
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?;

    if !openai_content.is_null() {
        return Some(openai_content.clone());
    }

    None
}

// ============================================================================
// Tool Result Processing
// ============================================================================

/// Process tool execution result and build next prompt
pub fn process_tool_result(
    state: &mut AgentState,
    tool_name: &str,
    success: bool,
    output: serde_json::Value,
    error: Option<String>,
) -> Result<String, String> {
    let result = if success {
        output.clone()
    } else {
        let err_msg = error.clone().unwrap_or_else(|| "Unknown error".to_string());
        serde_json::json!({"error": err_msg})
    };

    state.record_tool_result(tool_name.to_string(), result);

    let message = if success {
        format!(
            "Tool '{}' executed successfully.\nResult: {}",
            tool_name,
            serde_json::to_string_pretty(&output).unwrap_or_default()
        )
    } else {
        let err_msg = error.clone().unwrap_or_else(|| "Unknown error".to_string());
        format!("Tool '{}' failed.\nError: {}", tool_name, err_msg)
    };

    state.add_message(AgentMessage {
        role: "tool".to_string(),
        content: message.clone(),
        tool_call: None,
        tool_result: Some(ToolResult {
            name: tool_name.to_string(),
            success,
            output,
            error,
            step_id: state.current_step,
        }),
    });

    Ok(message)
}

// ============================================================================
// Prompt Building
// ============================================================================

/// Build system prompt for LLM
pub fn build_system_prompt(template: &str, variables: &PromptVariables) -> String {
    variables.render(template)
}

/// Build messages for LLM API (for host to use)
pub fn build_llm_messages(system_prompt: &str, state: &AgentState) -> Vec<HashMap<String, String>> {
    let mut messages = Vec::new();

    messages.push(HashMap::from([
        ("role".to_string(), "system".to_string()),
        ("content".to_string(), system_prompt.to_string()),
    ]));

    if let Some(summary) = &state.rolling_summary {
        messages.push(HashMap::from([
            ("role".to_string(), "system".to_string()),
            (
                "content".to_string(),
                format!("Conversation summary v{}: {}", summary.version, summary.text),
            ),
        ]));
    }

    for msg in &state.message_history {
        let mut message = HashMap::from([
            ("role".to_string(), msg.role.clone()),
            ("content".to_string(), msg.content.clone()),
        ]);

        if let Some(tool_call) = &msg.tool_call {
            message.insert(
                "tool_calls".to_string(),
                serde_json::to_string(&tool_call).unwrap_or_default(),
            );
        }

        messages.push(message);
    }

    messages
}

// ============================================================================
// Validation
// ============================================================================

/// Validate JSON against schema
pub fn validate_json_schema(schema: &serde_json::Value, data: &serde_json::Value) -> Result<(), String> {
    if data.is_null() {
        return Err("JSON is null".to_string());
    }

    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        if let Some(obj) = data.as_object() {
            for field in required {
                if let Some(field_str) = field.as_str() {
                    if !obj.contains_key(field_str) {
                        return Err(format!("Missing required field: {}", field_str));
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_native_tool_call() {
        let mut state = AgentState::new(AgentConfig::default());
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "weather.get",
                            "arguments": "{\"city\":\"Jakarta\"}"
                        }
                    }]
                }
            }]
        })
        .to_string();

        let action = process_llm_response(&mut state, &response).unwrap();
        match action {
            AgentAction::CallTool { tool, input } => {
                assert_eq!(tool, "weather.get");
                assert_eq!(input["city"], "Jakarta");
            }
            _ => panic!("expected call tool"),
        }
    }

    #[test]
    fn parses_anthropic_native_tool_use() {
        let mut state = AgentState::new(AgentConfig::default());
        let response = serde_json::json!({
            "content": [
                {
                    "type": "tool_use",
                    "name": "math.sum",
                    "input": {"a": 3, "b": 5}
                }
            ]
        })
        .to_string();

        let action = process_llm_response(&mut state, &response).unwrap();
        match action {
            AgentAction::CallTool { tool, input } => {
                assert_eq!(tool, "math.sum");
                assert_eq!(input["a"], 3);
                assert_eq!(input["b"], 5);
            }
            _ => panic!("expected call tool"),
        }
    }
}
