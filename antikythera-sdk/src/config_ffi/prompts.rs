//! Prompt Management FFI
//!
//! Get, set, and list prompt templates.

use std::os::raw::c_char;
use super::config;
use super::helpers::*;

/// Get a specific prompt template by name
///
/// # Parameters
/// - `name`: Prompt template name (see list_prompts for available names)
///
/// # Returns
/// JSON with `name` and `value` fields
pub fn mcp_config_get_prompt(name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    match get_prompt_value(&cfg.prompts, &name_str) {
        Ok(value) => serialize_result(&serde_json::json!({
            "name": name_str,
            "value": value
        })),
        Err(e) => error_response(&e),
    }
}

/// Set a specific prompt template
///
/// # Parameters
/// - `name`: Prompt template name
/// - `value`: New template content
///
/// # Returns
/// JSON with `success` and `name` fields
pub fn mcp_config_set_prompt(name: *const c_char, value: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let value_str = match from_c_string(value) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    match set_prompt_value(&mut cfg.prompts, &name_str, &value_str) {
        Ok(()) => match config::save_config(&cfg, None) {
            Ok(()) => serialize_result(&serde_json::json!({
                "success": true,
                "name": name_str
            })),
            Err(e) => error_response(&e),
        },
        Err(e) => error_response(&e),
    }
}

/// List all available prompt template names
///
/// # Returns
/// JSON array of prompt names
pub fn mcp_config_list_prompts() -> *mut c_char {
    serialize_result(&vec![
        "template", "tool_guidance", "fallback_guidance",
        "json_retry_message", "tool_result_instruction",
        "agent_instructions", "ui_instructions",
        "language_instructions", "agent_max_steps_error",
        "no_tools_guidance", "fallback_response_keys"
    ])
}

// ============================================================================
// Internal Helpers
// ============================================================================

pub fn get_prompt_value(prompts: &config::PromptsConfig, name: &str) -> Result<String, String> {
    match name {
        "template" => Ok(prompts.template.clone()),
        "tool_guidance" => Ok(prompts.tool_guidance.clone()),
        "fallback_guidance" => Ok(prompts.fallback_guidance.clone()),
        "json_retry_message" => Ok(prompts.json_retry_message.clone()),
        "tool_result_instruction" => Ok(prompts.tool_result_instruction.clone()),
        "agent_instructions" => Ok(prompts.agent_instructions.clone()),
        "ui_instructions" => Ok(prompts.ui_instructions.clone()),
        "language_instructions" => Ok(prompts.language_instructions.clone()),
        "agent_max_steps_error" => Ok(prompts.agent_max_steps_error.clone()),
        "no_tools_guidance" => Ok(prompts.no_tools_guidance.clone()),
        "fallback_response_keys" => Ok(prompts.fallback_response_keys.join(",")),
        _ => Err(format!("Unknown prompt field: {}", name)),
    }
}

pub fn set_prompt_value(prompts: &mut config::PromptsConfig, name: &str, value: &str) -> Result<(), String> {
    match name {
        "template" => prompts.template = value.to_string(),
        "tool_guidance" => prompts.tool_guidance = value.to_string(),
        "fallback_guidance" => prompts.fallback_guidance = value.to_string(),
        "json_retry_message" => prompts.json_retry_message = value.to_string(),
        "tool_result_instruction" => prompts.tool_result_instruction = value.to_string(),
        "agent_instructions" => prompts.agent_instructions = value.to_string(),
        "ui_instructions" => prompts.ui_instructions = value.to_string(),
        "language_instructions" => prompts.language_instructions = value.to_string(),
        "agent_max_steps_error" => prompts.agent_max_steps_error = value.to_string(),
        "no_tools_guidance" => prompts.no_tools_guidance = value.to_string(),
        "fallback_response_keys" => {
            prompts.fallback_response_keys = value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        _ => return Err(format!("Unknown prompt field: {}", name)),
    }
    Ok(())
}

