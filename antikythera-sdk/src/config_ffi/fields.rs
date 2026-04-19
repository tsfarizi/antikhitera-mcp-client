//! Field-Level Configuration FFI
//!
//! Get and set individual configuration fields by path.

use std::os::raw::c_char;
use super::config;
use super::helpers::*;

/// Get a specific configuration field by path
///
/// # Parameters
/// - `field`: Dot-separated field path (e.g., "server.bind", "model.default_provider")
///
/// # Returns
/// JSON with `field` and `value` fields
///
/// # Supported Paths
/// - `server.bind`
/// - `server.cors_origins`
/// - `model.default_provider`
/// - `model.model`
/// - `agent.max_steps`
/// - `agent.verbose`
/// - `agent.auto_execute_tools`
/// - `agent.session_timeout_secs`
/// - `prompts.<name>` (all prompt fields)
/// - `providers` (all providers as JSON array)
pub fn mcp_config_get(field: *const c_char) -> *mut c_char {
    let field_str = match from_c_string(field) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    match get_field_value(&cfg, &field_str) {
        Ok(value) => serialize_result(&serde_json::json!({
            "field": field_str,
            "value": value
        })),
        Err(e) => error_response(&e),
    }
}

/// Set a specific configuration field by path
///
/// # Parameters
/// - `field`: Dot-separated field path
/// - `value`: New value (JSON-encoded for complex types)
///
/// # Returns
/// JSON with `success`, `field`, and `value` fields
pub fn mcp_config_set(field: *const c_char, value: *const c_char) -> *mut c_char {
    let field_str = match from_c_string(field) {
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

    match set_field_value(&mut cfg, &field_str, &value_str) {
        Ok(()) => match config::save_config(&cfg, None) {
            Ok(()) => serialize_result(&serde_json::json!({
                "success": true,
                "field": field_str,
                "value": value_str
            })),
            Err(e) => error_response(&e),
        },
        Err(e) => error_response(&e),
    }
}

// ============================================================================
// Internal Field Access Helpers
// ============================================================================

pub fn get_field_value(cfg: &config::AppConfig, field: &str) -> Result<String, String> {
    match field {
        "server.bind" => Ok(cfg.server.bind.clone()),
        "server.cors_origins" => Ok(serde_json::to_string(&cfg.server.cors_origins).unwrap()),
        "model.default_provider" => Ok(cfg.model.default_provider.clone()),
        "model.model" => Ok(cfg.model.model.clone()),
        "agent.max_steps" => Ok(cfg.agent.max_steps.to_string()),
        "agent.verbose" => Ok(cfg.agent.verbose.to_string()),
        "agent.auto_execute_tools" => Ok(cfg.agent.auto_execute_tools.to_string()),
        "agent.session_timeout_secs" => Ok(cfg.agent.session_timeout_secs.to_string()),
        "providers" => Ok(serde_json::to_string(&cfg.providers).unwrap()),
        _ if field.starts_with("prompts.") => {
            let name = field.trim_start_matches("prompts.");
            get_prompt_value(&cfg.prompts, name)
        }
        _ => Err(format!("Unknown field: {}", field)),
    }
}

pub fn set_field_value(cfg: &mut config::AppConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "server.bind" => {
            cfg.server.bind = value.to_string();
            Ok(())
        }
        "server.cors_origins" => {
            cfg.server.cors_origins = serde_json::from_str(value)
                .map_err(|e| format!("Invalid JSON: {}", e))?;
            Ok(())
        }
        "model.default_provider" => {
            cfg.model.default_provider = value.to_string();
            Ok(())
        }
        "model.model" => {
            cfg.model.model = value.to_string();
            Ok(())
        }
        "agent.max_steps" => {
            cfg.agent.max_steps = value.parse()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(())
        }
        "agent.verbose" => {
            cfg.agent.verbose = value.parse()
                .map_err(|e| format!("Invalid bool: {}", e))?;
            Ok(())
        }
        "agent.auto_execute_tools" => {
            cfg.agent.auto_execute_tools = value.parse()
                .map_err(|e| format!("Invalid bool: {}", e))?;
            Ok(())
        }
        "agent.session_timeout_secs" => {
            cfg.agent.session_timeout_secs = value.parse()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(())
        }
        _ if field.starts_with("prompts.") => {
            let name = field.trim_start_matches("prompts.");
            set_prompt_value(&mut cfg.prompts, name, value)
        }
        _ => Err(format!("Unknown field: {}", field)),
    }
}

fn get_prompt_value(prompts: &config::PromptsConfig, name: &str) -> Result<String, String> {
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
        _ => Err(format!("Unknown prompt field: {}", name)),
    }
}

fn set_prompt_value(prompts: &mut config::PromptsConfig, name: &str, value: &str) -> Result<(), String> {
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
        _ => return Err(format!("Unknown prompt field: {}", name)),
    }
    Ok(())
}

