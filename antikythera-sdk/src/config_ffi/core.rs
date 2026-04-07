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

/// Load configuration from a custom file path
///
/// This copies the config from the specified path to the default location (app.pc).
/// Useful for restoring config when rebuilding infrastructure.
///
/// # Parameters
/// - `source_path`: Path to existing .pc config file
///
/// # Returns
/// JSON with `success`, `source`, `destination`, and `size_bytes` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_use_from(source_path: *const c_char) -> *mut c_char {
    let source_str = match from_c_string(source_path) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let source = std::path::Path::new(&source_str);

    // Verify source exists
    if !source.exists() {
        return error_response(&format!("Source config not found: {}", source_str));
    }

    // Copy to default location
    let dest = std::path::Path::new(config::CONFIG_PATH);

    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return error_response(&format!("Failed to create config directory: {}", e));
            }
        }
    }

    match std::fs::copy(source, dest) {
        Ok(size) => {
            // Verify the loaded config is valid
            match config::load_config(None) {
                Ok(_) => success_with(&[
                    ("source", serde_json::json!(source_str)),
                    ("destination", serde_json::json!(config::CONFIG_PATH)),
                    ("size_bytes", serde_json::json!(size)),
                ]),
                Err(e) => success_with(&[
                    ("source", serde_json::json!(source_str)),
                    ("destination", serde_json::json!(config::CONFIG_PATH)),
                    ("size_bytes", serde_json::json!(size)),
                    ("warning", serde_json::json!(format!("Config copied but may be corrupted: {}", e))),
                ]),
            }
        }
        Err(e) => error_response(&format!("Failed to copy config: {}", e)),
    }
}

/// Backup current configuration to a custom file path
///
/// # Parameters
/// - `dest_path`: Path to save backup (e.g., "my-backup.pc")
///
/// # Returns
/// JSON with `success`, `path`, and `size_bytes` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_backup_to(dest_path: *const c_char) -> *mut c_char {
    let dest_str = match from_c_string(dest_path) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Load current config
    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to load config: {}", e)),
    };

    // Save to custom path
    match config::save_config(&cfg, Some(std::path::Path::new(&dest_str))) {
        Ok(()) => {
            let size = std::fs::metadata(&dest_str)
                .map(|m| m.len())
                .unwrap_or(0);

            success_with(&[
                ("path", serde_json::json!(dest_str)),
                ("size_bytes", serde_json::json!(size)),
            ])
        }
        Err(e) => error_response(&e),
    }
}
