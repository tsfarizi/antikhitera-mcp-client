//! Provider Management FFI
//!
//! Add, remove, and list LLM providers.

use std::os::raw::c_char;
use super::config;
use super::helpers::*;

/// Add a new LLM provider
///
/// # Parameters
/// - `id`: Unique provider identifier (e.g., "openai")
/// - `provider_type`: Provider type (e.g., "openai", "anthropic")
/// - `endpoint`: API endpoint URL
/// - `api_key`: Environment variable name for API key
///
/// # Returns
/// JSON with `success` and `provider_id` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_add_provider(
    id: *const c_char,
    provider_type: *const c_char,
    endpoint: *const c_char,
    api_key: *const c_char,
) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let type_str = match from_c_string(provider_type) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let endpoint_str = match from_c_string(endpoint) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let api_key_str = match from_c_string(api_key) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    // Check for duplicates
    if cfg.providers.iter().any(|p| p.id == id_str) {
        return error_response(&format!("Provider '{}' already exists", id_str));
    }

    cfg.providers.push(config::ProviderConfig {
        id: id_str.clone(),
        provider_type: type_str,
        endpoint: endpoint_str,
        api_key: api_key_str,
        models: Vec::new(),
    });

    match config::save_config(&cfg, None) {
        Ok(()) => success_with(&[
            ("provider_id", serde_json::json!(id_str)),
        ]),
        Err(e) => error_response(&e),
    }
}

/// Remove an LLM provider by ID
///
/// # Parameters
/// - `id`: Provider ID to remove
///
/// # Returns
/// JSON with `success` and `provider_id` fields, or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_remove_provider(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    let initial_len = cfg.providers.len();
    cfg.providers.retain(|p| p.id != id_str);

    if cfg.providers.len() == initial_len {
        return error_response(&format!("Provider '{}' not found", id_str));
    }

    match config::save_config(&cfg, None) {
        Ok(()) => success_with(&[
            ("provider_id", serde_json::json!(id_str)),
        ]),
        Err(e) => error_response(&e),
    }
}

/// List all configured LLM providers
///
/// # Returns
/// JSON array of ProviderConfig objects
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_list_providers() -> *mut c_char {
    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    serialize_result(&cfg.providers)
}
