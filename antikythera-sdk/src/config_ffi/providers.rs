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

/// Set/update the API key for an existing provider
///
/// # Parameters
/// - `id`: Provider ID
/// - `api_key`: New API key or environment variable name
///
/// # Returns
/// JSON with `success`, `provider_id`, and `api_key` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_set_provider_api_key(
    id: *const c_char,
    api_key: *const c_char,
) -> *mut c_char {
    let id_str = match from_c_string(id) {
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

    // Find and update the provider
    let provider = cfg.providers.iter_mut().find(|p| p.id == id_str);
    match provider {
        Some(p) => {
            p.api_key = api_key_str.clone();
            match config::save_config(&cfg, None) {
                Ok(()) => success_with(&[
                    ("provider_id", serde_json::json!(id_str)),
                    ("api_key", serde_json::json!(api_key_str)),
                ]),
                Err(e) => error_response(&e),
            }
        }
        None => error_response(&format!("Provider '{}' not found", id_str)),
    }
}

/// Get the API key for a specific provider
///
/// # Parameters
/// - `id`: Provider ID
///
/// # Returns
/// JSON with `provider_id` and `api_key` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_get_provider_api_key(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    match cfg.providers.iter().find(|p| p.id == id_str) {
        Some(p) => serialize_result(&serde_json::json!({
            "provider_id": p.id,
            "api_key": p.api_key,
        })),
        None => error_response(&format!("Provider '{}' not found", id_str)),
    }
}

/// Add a model to an existing provider
///
/// # Parameters
/// - `provider_id`: Provider ID
/// - `model_name`: Model identifier (e.g., "gpt-4")
/// - `display_name`: Human-readable name (e.g., "GPT-4")
///
/// # Returns
/// JSON with `success`, `provider_id`, and `model` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_add_provider_model(
    provider_id: *const c_char,
    model_name: *const c_char,
    display_name: *const c_char,
) -> *mut c_char {
    let provider_id_str = match from_c_string(provider_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let model_name_str = match from_c_string(model_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let display_name_str = match from_c_string(display_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    let provider = cfg.providers.iter_mut().find(|p| p.id == provider_id_str);
    match provider {
        Some(p) => {
            // Check if model already exists
            if p.models.iter().any(|m| m.name == model_name_str) {
                return error_response(&format!("Model '{}' already exists for provider '{}'", model_name_str, provider_id_str));
            }

            p.models.push(config::ModelInfo {
                name: model_name_str.clone(),
                display_name: display_name_str.clone(),
            });

            match config::save_config(&cfg, None) {
                Ok(()) => success_with(&[
                    ("provider_id", serde_json::json!(provider_id_str)),
                    ("model", serde_json::json!(model_name_str)),
                    ("display_name", serde_json::json!(display_name_str)),
                ]),
                Err(e) => error_response(&e),
            }
        }
        None => error_response(&format!("Provider '{}' not found", provider_id_str)),
    }
}

/// Remove a model from a provider
///
/// # Parameters
/// - `provider_id`: Provider ID
/// - `model_name`: Model name to remove
///
/// # Returns
/// JSON with `success`, `provider_id`, and `model` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_remove_provider_model(
    provider_id: *const c_char,
    model_name: *const c_char,
) -> *mut c_char {
    let provider_id_str = match from_c_string(provider_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let model_name_str = match from_c_string(model_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    let provider = cfg.providers.iter_mut().find(|p| p.id == provider_id_str);
    match provider {
        Some(p) => {
            let initial_len = p.models.len();
            p.models.retain(|m| m.name != model_name_str);

            if p.models.len() == initial_len {
                return error_response(&format!("Model '{}' not found for provider '{}'", model_name_str, provider_id_str));
            }

            match config::save_config(&cfg, None) {
                Ok(()) => success_with(&[
                    ("provider_id", serde_json::json!(provider_id_str)),
                    ("model", serde_json::json!(model_name_str)),
                ]),
                Err(e) => error_response(&e),
            }
        }
        None => error_response(&format!("Provider '{}' not found", provider_id_str)),
    }
}

/// List all models for a specific provider
///
/// # Parameters
/// - `provider_id`: Provider ID
///
/// # Returns
/// JSON array of model objects with `name` and `display_name`
#[unsafe(no_mangle)]
pub extern "C" fn mcp_config_list_provider_models(provider_id: *const c_char) -> *mut c_char {
    let provider_id_str = match from_c_string(provider_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    match cfg.providers.iter().find(|p| p.id == provider_id_str) {
        Some(p) => serialize_result(&serde_json::json!({
            "provider_id": p.id,
            "models": p.models,
        })),
        None => error_response(&format!("Provider '{}' not found", provider_id_str)),
    }
}
