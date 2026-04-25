//! Secrets Management FFI

use super::helpers::*;
use crate::sdk_logging::SecurityFfiLogger;
use antikythera_core::security::secrets::SecretManager;
use std::os::raw::c_char;
use std::sync::Mutex;

/// Global secret manager instance (thread-safe)
static SECRET_MANAGER: Mutex<Option<SecretManager>> = Mutex::new(None);

/// Initialize the secret manager with default configuration
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_init_secret_manager() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_init_secret_manager", "{}");

    match SecretManager::from_config() {
        Ok(manager) => {
            let mut guard = SECRET_MANAGER.lock().unwrap();
            *guard = Some(manager);
            logger.ffi_result("mcp_security_init_secret_manager", true, 0);
            success_response()
        }
        Err(e) => {
            logger.ffi_error("mcp_security_init_secret_manager", &e.to_string());
            error_response(&e.to_string())
        }
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
pub extern "C" fn mcp_security_store_secret(
    id: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_store_secret", &e);
            return error_response(&e);
        }
    };

    let value_str = match from_c_string(value) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_store_secret", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_store_secret",
        &format!("{{\"id\": \"{}\"}}", id_str),
    );

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => match manager.store_secret(&id_str, &value_str) {
            Ok(_) => {
                logger.secret_stored(&id_str);
                logger.ffi_result("mcp_security_store_secret", true, 0);
                success_response()
            }
            Err(e) => {
                logger.ffi_error("mcp_security_store_secret", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_store_secret",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
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
    let logger = SecurityFfiLogger::new("security");
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_get_secret", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_get_secret",
        &format!("{{\"id\": \"{}\"}}", id_str),
    );

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => match manager.get_secret(&id_str) {
            Ok(value) => {
                logger.secret_retrieved(&id_str);
                logger.ffi_result("mcp_security_get_secret", true, value.len());
                success_with(&[("value", serde_json::json!(value))])
            }
            Err(e) => {
                logger.ffi_error("mcp_security_get_secret", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error("mcp_security_get_secret", "Secret manager not initialized");
            error_response("Secret manager not initialized")
        }
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
pub extern "C" fn mcp_security_rotate_secret(
    id: *const c_char,
    new_value: *const c_char,
) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_rotate_secret", &e);
            return error_response(&e);
        }
    };

    let new_value_str = match from_c_string(new_value) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_rotate_secret", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_rotate_secret",
        &format!("{{\"id\": \"{}\"}}", id_str),
    );

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => match manager.rotate_secret(&id_str, &new_value_str) {
            Ok(_) => {
                logger.secret_rotated(&id_str);
                logger.ffi_result("mcp_security_rotate_secret", true, 0);
                success_response()
            }
            Err(e) => {
                logger.ffi_error("mcp_security_rotate_secret", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_rotate_secret",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
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
    let logger = SecurityFfiLogger::new("security");
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_delete_secret", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_delete_secret",
        &format!("{{\"id\": \"{}\"}}", id_str),
    );

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => match manager.delete_secret(&id_str) {
            Ok(_) => {
                logger.secret_deleted(&id_str);
                logger.ffi_result("mcp_security_delete_secret", true, 0);
                success_response()
            }
            Err(e) => {
                logger.ffi_error("mcp_security_delete_secret", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_delete_secret",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
    }
}

/// List all secret IDs
///
/// # Returns
/// JSON with `success` and `secrets` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_list_secrets() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_list_secrets", "{}");

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            let secrets = manager.list_secrets();
            logger.ffi_result("mcp_security_list_secrets", true, secrets.len());
            success_with(&[("secrets", serde_json::json!(secrets))])
        }
        None => {
            logger.ffi_error(
                "mcp_security_list_secrets",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
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
    let logger = SecurityFfiLogger::new("security");
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_get_secret_metadata", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_get_secret_metadata",
        &format!("{{\"id\": \"{}\"}}", id_str),
    );

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => match manager.get_metadata(&id_str) {
            Ok(metadata) => {
                logger.ffi_result("mcp_security_get_secret_metadata", true, 0);
                serialize_result(&metadata)
            }
            Err(e) => {
                logger.ffi_error("mcp_security_get_secret_metadata", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_get_secret_metadata",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
    }
}

/// Get current secrets configuration
///
/// # Returns
/// JSON with current secrets configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_secrets_config() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_get_secrets_config", "{}");

    let guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_ref() {
        Some(manager) => {
            logger.ffi_result("mcp_security_get_secrets_config", true, 0);
            serialize_result(manager.config())
        }
        None => {
            logger.ffi_error(
                "mcp_security_get_secrets_config",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
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
    let logger = SecurityFfiLogger::new("security");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_set_secrets_config", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call("mcp_security_set_secrets_config", &json_str);

    let config: antikythera_core::security::config::SecretsConfig =
        match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => {
                let err = format!("Invalid JSON: {}", e);
                logger.ffi_error("mcp_security_set_secrets_config", &err);
                return error_response(&err);
            }
        };

    let mut guard = SECRET_MANAGER.lock().unwrap();
    match guard.as_mut() {
        Some(manager) => match manager.update_config(config) {
            Ok(_) => {
                logger.ffi_result("mcp_security_set_secrets_config", true, 0);
                success_response()
            }
            Err(e) => {
                logger.ffi_error("mcp_security_set_secrets_config", &e.to_string());
                error_response(&e.to_string())
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_set_secrets_config",
                "Secret manager not initialized",
            );
            error_response("Secret manager not initialized")
        }
    }
}
