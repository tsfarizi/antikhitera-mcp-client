//! Prompt Management Feature Slice
//!
//! Provides FFI bindings for managing prompt templates via Postcard config.

use antikythera_core::config::postcard_config;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

fn load_config() -> Result<postcard_config::PostcardAppConfig, String> {
    postcard_config::load_config(None)
}

fn save_config(config: &postcard_config::PostcardAppConfig) -> Result<(), String> {
    postcard_config::save_config(config, None)
}

/// Get the main system prompt template
pub fn mcp_get_template() -> *mut c_char {
    match load_config() {
        Ok(config) => to_c_string(&config.prompts.template),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Update the main system prompt template
pub fn mcp_update_template(template: *const c_char) -> *mut c_char {
    let template_str = match from_c_string(template) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    let mut config = match load_config() {
        Ok(c) => c,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    config.prompts.template = template_str;

    match save_config(&config) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Reset the main template to default
pub fn mcp_reset_template() -> *mut c_char {
    let mut config = match load_config() {
        Ok(c) => c,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    config.prompts.template = postcard_config::PromptsConfig::default_template().to_string();

    match save_config(&config) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get tool guidance prompt
pub fn mcp_get_tool_guidance() -> *mut c_char {
    match load_config() {
        Ok(config) => to_c_string(&config.prompts.tool_guidance),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Update tool guidance prompt
pub fn mcp_update_tool_guidance(guidance: *const c_char) -> *mut c_char {
    let guidance_str = match from_c_string(guidance) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    let mut config = match load_config() {
        Ok(c) => c,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    config.prompts.tool_guidance = guidance_str;

    match save_config(&config) {
        Ok(()) => to_c_string(r#"{"success": true}"#),
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get all prompts as JSON object
pub fn mcp_get_all_prompts() -> *mut c_char {
    match load_config() {
        Ok(config) => {
            let json = serde_json::json!({
                "template": config.prompts.template,
                "tool_guidance": config.prompts.tool_guidance,
                "fallback_guidance": config.prompts.fallback_guidance,
                "json_retry_message": config.prompts.json_retry_message,
                "tool_result_instruction": config.prompts.tool_result_instruction,
                "agent_instructions": config.prompts.agent_instructions,
                "ui_instructions": config.prompts.ui_instructions,
                "language_instructions": config.prompts.language_instructions,
                "agent_max_steps_error": config.prompts.agent_max_steps_error,
                "no_tools_guidance": config.prompts.no_tools_guidance
            });
            to_c_string(&json.to_string())
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}
