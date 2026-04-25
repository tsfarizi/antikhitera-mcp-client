//! Input Validation FFI

use super::helpers::*;
use crate::sdk_logging::SecurityFfiLogger;
use antikythera_core::security::validation::{InputValidator, ValidationResult};
use std::os::raw::c_char;
use std::sync::Mutex;

/// Global validator instance (thread-safe)
static VALIDATOR: Mutex<Option<InputValidator>> = Mutex::new(None);

/// Initialize the input validator with default configuration
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_init_validator() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_init_validator", "{}");

    match InputValidator::from_config() {
        Ok(validator) => {
            let mut guard = VALIDATOR.lock().unwrap();
            *guard = Some(validator);
            logger.ffi_result("mcp_security_init_validator", true, 0);
            success_response()
        }
        Err(e) => {
            logger.ffi_error("mcp_security_init_validator", &e);
            error_response(&e)
        }
    }
}

/// Validate input string
///
/// # Parameters
/// - `input`: Input string to validate
///
/// # Returns
/// JSON with `success`, `valid`, and optional `error` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_validate_input(input: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let input_str = match from_c_string(input) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_validate_input", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_validate_input",
        &format!("{{\"input_size\": {}}}", input_str.len()),
    );

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => match validator.validate(&input_str) {
            Ok(_) => {
                logger.validation_passed("input");
                logger.ffi_result("mcp_security_validate_input", true, 0);
                success_with(&[("valid", serde_json::json!(true))])
            }
            Err(errors) => {
                let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                let reason = error_messages.join(", ");
                logger.validation_failed("input", &reason);
                logger.ffi_result("mcp_security_validate_input", false, 0);
                success_with(&[
                    ("valid", serde_json::json!(false)),
                    ("errors", serde_json::json!(error_messages)),
                ])
            }
        },
        None => {
            logger.ffi_error("mcp_security_validate_input", "Validator not initialized");
            error_response("Validator not initialized")
        }
    }
}

/// Validate URL
///
/// # Parameters
/// - `url`: URL string to validate
///
/// # Returns
/// JSON with `success`, `valid`, and optional `error` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_validate_url(url: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let url_str = match from_c_string(url) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_validate_url", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_validate_url",
        &format!("{{\"url\": \"{}\"}}", url_str),
    );

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => match validator.validate_url(&url_str) {
            ValidationResult::Valid => {
                logger.validation_passed("url");
                logger.ffi_result("mcp_security_validate_url", true, 0);
                success_with(&[("valid", serde_json::json!(true))])
            }
            ValidationResult::Invalid(msg) => {
                logger.validation_failed("url", &msg);
                logger.ffi_result("mcp_security_validate_url", false, 0);
                success_with(&[
                    ("valid", serde_json::json!(false)),
                    ("error", serde_json::json!(msg)),
                ])
            }
            ValidationResult::Sanitized(sanitized) => {
                logger.validation_passed("url (sanitized)");
                logger.ffi_result("mcp_security_validate_url", true, sanitized.len());
                success_with(&[
                    ("valid", serde_json::json!(true)),
                    ("sanitized", serde_json::json!(sanitized)),
                ])
            }
        },
        None => {
            logger.ffi_error("mcp_security_validate_url", "Validator not initialized");
            error_response("Validator not initialized")
        }
    }
}

/// Validate JSON structure
///
/// # Parameters
/// - `json_str`: JSON string to validate
///
/// # Returns
/// JSON with `success`, `valid`, and optional `error` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_validate_json(json_str: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let json = match from_c_string(json_str) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_validate_json", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_validate_json",
        &format!("{{\"json_size\": {}}}", json.len()),
    );

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => match validator.validate_json(&json) {
            Ok(_) => {
                logger.validation_passed("json");
                logger.ffi_result("mcp_security_validate_json", true, 0);
                success_with(&[("valid", serde_json::json!(true))])
            }
            Err(e) => {
                logger.validation_failed("json", &e);
                logger.ffi_result("mcp_security_validate_json", false, 0);
                success_with(&[
                    ("valid", serde_json::json!(false)),
                    ("error", serde_json::json!(e)),
                ])
            }
        },
        None => {
            logger.ffi_error("mcp_security_validate_json", "Validator not initialized");
            error_response("Validator not initialized")
        }
    }
}

/// Sanitize HTML content
///
/// # Parameters
/// - `html`: HTML string to sanitize
///
/// # Returns
/// JSON with `success` and `sanitized` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_sanitize_html(html: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let html_str = match from_c_string(html) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_sanitize_html", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_sanitize_html",
        &format!("{{\"html_size\": {}}}", html_str.len()),
    );

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            let sanitized = validator.sanitize_html(&html_str);
            logger.ffi_result("mcp_security_sanitize_html", true, sanitized.len());
            success_with(&[("sanitized", serde_json::json!(sanitized))])
        }
        None => {
            logger.ffi_error("mcp_security_sanitize_html", "Validator not initialized");
            error_response("Validator not initialized")
        }
    }
}

/// Get current validation configuration
///
/// # Returns
/// JSON with current validation configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_validation_config() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_get_validation_config", "{}");

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            let config = validator.config();
            logger.ffi_result("mcp_security_get_validation_config", true, 0);
            serialize_result(config)
        }
        None => {
            logger.ffi_error(
                "mcp_security_get_validation_config",
                "Validator not initialized",
            );
            error_response("Validator not initialized")
        }
    }
}

/// Set validation configuration
///
/// # Parameters
/// - `config_json`: Validation configuration as JSON string
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_set_validation_config(config_json: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_set_validation_config", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call("mcp_security_set_validation_config", &json_str);

    let config: antikythera_core::security::config::ValidationConfig =
        match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => {
                let err = format!("Invalid JSON: {}", e);
                logger.ffi_error("mcp_security_set_validation_config", &err);
                return error_response(&err);
            }
        };

    let mut guard = VALIDATOR.lock().unwrap();
    match guard.as_mut() {
        Some(validator) => match validator.update_config(config) {
            Ok(_) => {
                logger.ffi_result("mcp_security_set_validation_config", true, 0);
                success_response()
            }
            Err(e) => {
                logger.ffi_error("mcp_security_set_validation_config", &e);
                error_response(&e)
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_set_validation_config",
                "Validator not initialized",
            );
            error_response("Validator not initialized")
        }
    }
}
