//! Core Configuration FFI
//!
//! Core operations: init, exists, size, get/set all, export, import, reset.

use std::os::raw::c_char;
use super::config;
use super::helpers::*;

/// Initialize default configuration and save as Postcard
///
/// # Returns
/// JSON with `success`, `path`, and `size_bytes` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_init() -> *mut c_char {
    if config::config_exists() {
        return error_response("Config already exists. Use reset to overwrite.");
    }

    match config::init_default_config(None) {
        Ok(_) => {
            let size = config::config_size(None).unwrap_or(0);
            success_with(&[
                ("path", serde_json::json!(config::CONFIG_PATH)),
                ("size_bytes", serde_json::json!(size)),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Check if configuration file exists
///
/// # Returns
/// 1 if exists, 0 if not
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_exists() -> i32 {
    if config::config_exists() { 1 } else { 0 }
}

/// Get configuration size in bytes
///
/// # Returns
/// JSON with `size_bytes` and `path` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_size() -> *mut c_char {
    match config::config_size(None) {
        Ok(size) => serialize_result(&serde_json::json!({
            "size_bytes": size,
            "path": config::CONFIG_PATH
        })),
        Err(e) => error_response(&e),
    }
}

/// Get entire configuration as JSON
///
/// # Returns
/// Full AppConfig serialized as JSON
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_get_all() -> *mut c_char {
    match config::load_config(None) {
        Ok(cfg) => serialize_result(&cfg),
        Err(e) => error_response(&e),
    }
}

/// Save entire configuration from JSON
///
/// # Parameters
/// - `config_json`: Full AppConfig as JSON string
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_set_all(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg: config::AppConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
    };

    match config::save_config(&cfg, None) {
        Ok(()) => success_response(),
        Err(e) => error_response(&e),
    }
}

/// Export configuration as pretty-printed JSON
///
/// # Returns
/// Formatted JSON string of entire config
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_export() -> *mut c_char {
    match config::load_config(None) {
        Ok(cfg) => match serde_json::to_string_pretty(&cfg) {
            Ok(json) => to_c_string(&json),
            Err(e) => error_response(&format!("Serialization failed: {}", e)),
        },
        Err(e) => error_response(&e),
    }
}

/// Import configuration from JSON string
///
/// # Parameters
/// - `config_json`: Full AppConfig as JSON string
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_import(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg: config::AppConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
    };

    match config::save_config(&cfg, None) {
        Ok(()) => success_response(),
        Err(e) => error_response(&e),
    }
}

/// Reset configuration to defaults
///
/// # Returns
/// JSON with `success`, `path`, and `size_bytes` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_reset() -> *mut c_char {
    match config::init_default_config(None) {
        Ok(_) => {
            let size = config::config_size(None).unwrap_or(0);
            success_with(&[
                ("path", serde_json::json!(config::CONFIG_PATH)),
                ("size_bytes", serde_json::json!(size)),
            ])
        }
        Err(e) => error_response(&e),
    }
}
