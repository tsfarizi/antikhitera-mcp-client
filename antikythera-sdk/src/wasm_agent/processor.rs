//! WASM Agent Processor
//!
//! Processes LLM responses and determines next action.
//! WASM does NOT call LLM APIs - host handles that.

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// LLM Response Processing
// ============================================================================

/// Process LLM response and determine next action
pub fn process_llm_response(
    state: &mut AgentState,
    llm_response_content: &str,
) -> Result<AgentAction, String> {
    // Check max steps
    if state.is_max_steps_exceeded() {
        return Err(format!(
            "Max steps exceeded: {} >= {}",
            state.current_step, state.config.max_steps
        ));
    }

    // Parse LLM response as JSON
    let parsed: serde_json::Value = serde_json::from_str(llm_response_content)
        .map_err(|e| format!("Invalid JSON from LLM: {}", e))?;

    // Determine action from response
    let action = parsed.get("action")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'action' field in LLM response")?;

    match action {
        "call_tool" => {
            let tool = parsed.get("tool")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'tool' field")?
                .to_string();

            let input = parsed.get("input")
                .or_else(|| parsed.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            // Record step
            state.current_step += 1;

            Ok(AgentAction::CallTool { tool, input })
        }

        "final" => {
            let response = parsed.get("response")
                .cloned()
                .ok_or("Missing 'response' field")?;

            Ok(AgentAction::Final { response })
        }

        _ => Err(format!("Unknown action: {}", action)),
    }
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
    // Record result
    let result = if success {
        output.clone()
    } else {
        let err_msg = error.clone().unwrap_or_else(|| "Unknown error".to_string());
        serde_json::json!({"error": err_msg})
    };

    state.record_tool_result(tool_name.to_string(), result);

    // Build tool result message for next LLM call
    let message = if success {
        format!(
            "Tool '{}' executed successfully.\nResult: {}",
            tool_name,
            serde_json::to_string_pretty(&output).unwrap_or_default()
        )
    } else {
        let err_msg = error.clone().unwrap_or_else(|| "Unknown error".to_string());
        format!(
            "Tool '{}' failed.\nError: {}",
            tool_name,
            err_msg
        )
    };

    // Add to message history
    state.add_message(Message {
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
pub fn build_system_prompt(
    template: &str,
    variables: &PromptVariables,
) -> String {
    variables.render(template)
}

/// Build messages for LLM API (for host to use)
pub fn build_llm_messages(
    system_prompt: &str,
    state: &AgentState,
) -> Vec<HashMap<String, String>> {
    let mut messages = Vec::new();

    // System message
    messages.push(HashMap::from([
        ("role".to_string(), "system".to_string()),
        ("content".to_string(), system_prompt.to_string()),
    ]));

    // Conversation history
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
pub fn validate_json_schema(
    schema: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<(), String> {
    // Basic validation - check if data is valid JSON
    if data.is_null() {
        return Err("JSON is null".to_string());
    }

    // If schema specifies required fields, check them
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
