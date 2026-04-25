//! Secrets Management FFI

use std::os::raw::c_char;
use std::sync::Mutex;
use super::helpers::*;
use antikythera_core::security::secrets::SecretManager;

/// Global secret manager instance (thread-safe)
static SECRET_MANAGER: Mutex<Option<SecretManager>> = Mutex::new(None);

/// Initialize the secret manager with default configuration
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_init_secret_manager() -> *mut c_char {
    match SecretManager::from_config() {
        Ok(manager) => {
            let mut guard = SECRET_MANAGER.lock().unwrap();
            *guard = Some(manager);
            success_response()
        }
        Err(e) => error_response(&e.to_string()),
    }
}

/// Store a secret
///
/// # Parameters
/// - `id`: Secret identifier
/// - `value`: Secret value
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_store_secret(id: *const c_char, value: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let value_str = match from_c_string(value) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            match manager.store_secret(&id_str, &value_str) {
                Ok(_) => success_response(),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// Retrieve a secret
///
/// # Parameters
/// - `id`: Secret identifier
///
/// # Returns
/// JSON with `success` and `value` fields, or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_secret(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            match manager.get_secret(&id_str) {
                Ok(value) => success_with(&[("value", serde_json::json!(value))]),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// Rotate a secret
///
/// # Parameters
/// - `id`: Secret identifier
/// - `new_value`: New secret value
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_rotate_secret(id: *const c_char, new_value: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let new_value_str = match from_c_string(new_value) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            match manager.rotate_secret(&id_str, &new_value_str) {
                Ok(_) => success_response(),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// Delete a secret
///
/// # Parameters
/// - `id`: Secret identifier
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_delete_secret(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            match manager.delete_secret(&id_str) {
                Ok(_) => success_response(),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// List all secret IDs
///
/// # Returns
/// JSON with `success` and `secrets` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_list_secrets() -> *mut c_char {
    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            let secrets = manager.list_secrets();
            success_with(&[("secrets", serde_json::json!(secrets))])
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// Get secret metadata
///
/// # Parameters
/// - `id`: Secret identifier
///
/// # Returns
/// JSON with secret metadata
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_secret_metadata(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            match manager.get_metadata(&id_str) {
                Ok(metadata) => serialize_result(&metadata),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}

/// Get current secrets configuration
///
/// # Returns
/// JSON with current secrets configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_secrets_config() -> *mut c_char {
    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => serialize_result(manager.config()),
        None => error_response("Secret manager not initialized"),
    }
}

/// Set secrets configuration
///
/// # Parameters
/// - `config_json`: Secrets configuration as JSON string
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_set_secrets_config(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let config: antikythera_core::security::config::SecretsConfig =
        match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
        };

    let mut guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_mut() {
        Some(manager) => {
            match manager.update_config(config) {
                Ok(_) => success_response(),
                Err(e) => error_response(&e.to_string()),
            }
        }
        None => error_response("Secret manager not initialized"),
    }
}