//! Prompt Management Feature Slice
//!
//! Provides FFI bindings for managing prompt templates.

use antikythera_core::config::app::PromptsConfig;
use antikythera_core::config::wizard::generators::model;
use antikythera_core::constants::CONFIG_PATH;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::fs;

const MODEL_TOML_PATH: &str = "model.toml";

fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn from_c_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Null pointer".to_string());
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

fn load_prompts_config() -> Result<PromptsConfig, String> {
    use antikythera_core::config::app::AppConfig;
    let config_path = Path::new(CONFIG_PATH);
    let config = AppConfig::load(Some(config_path))
        .map_err(|e| format!("Failed to load config: {}", e))?;
    Ok(config.prompts)
}

/// Get the main system prompt template
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_template() -> *mut c_char {
    match load_prompts_config() {
        Ok(config) => to_c_string(config.template()),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Update the main system prompt template
#[unsafe(no_mangle)]
pub extern "C" fn mcp_update_template(template: *const c_char) -> *mut c_char {
    let template_str = match from_c_string(template) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match model::update_prompt_template(&template_str) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Reset the main template to default
#[unsafe(no_mangle)]
pub extern "C" fn mcp_reset_template() -> *mut c_char {
    match model::update_prompt_template(PromptsConfig::default_template()) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get tool guidance prompt
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_tool_guidance() -> *mut c_char {
    match load_prompts_config() {
        Ok(config) => to_c_string(config.tool_guidance()),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Update tool guidance prompt
#[unsafe(no_mangle)]
pub extern "C" fn mcp_update_tool_guidance(guidance: *const c_char) -> *mut c_char {
    let guidance_str = match from_c_string(guidance) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match model::update_tool_guidance(&guidance_str) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get all prompts as JSON object
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_all_prompts() -> *mut c_char {
    match load_prompts_config() {
        Ok(config) => {
            let json = serde_json::json!({
                "template": config.template(),
                "tool_guidance": config.tool_guidance(),
                "fallback_guidance": config.fallback_guidance(),
                "json_retry_message": config.json_retry_message(),
                "tool_result_instruction": config.tool_result_instruction(),
                "agent_instructions": config.agent_instructions(),
                "ui_instructions": config.ui_instructions(),
                "language_instructions": config.language_instructions(),
                "agent_max_steps_error": config.agent_max_steps_error(),
                "no_tools_guidance": config.no_tools_guidance()
            });
            to_c_string(&json.to_string())
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get the raw model.toml file content
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_raw_config() -> *mut c_char {
    let config_path = Path::new(LEGACY_MODEL_TOML);
    match fs::read_to_string(config_path) {
        Ok(content) => to_c_string(&content),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}
