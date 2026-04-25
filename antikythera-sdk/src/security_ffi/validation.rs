//! Input Validation FFI

use std::os::raw::c_char;
use std::sync::Mutex;
use super::helpers::*;
use antikythera_core::security::validation::{InputValidator, ValidationResult};

/// Global validator instance (thread-safe)
static VALIDATOR: Mutex<Option<InputValidator>> = Mutex::new(None);

/// Initialize the input validator with default configuration
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_init_validator() -> *mut c_char {
    match InputValidator::from_config() {
        Ok(validator) => {
            let mut guard = VALIDATOR.lock().unwrap();
            *guard = Some(validator);
            success_response()
        }
        Err(e) => error_response(&e),
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
    let input_str = match from_c_string(input) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            match validator.validate(&input_str) {
                Ok(_) => success_with(&[("valid", serde_json::json!(true))]),
                Err(errors) => {
                    let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                    success_with(&[
                        ("valid", serde_json::json!(false)),
                        ("errors", serde_json::json!(error_messages)),
                    ])
                }
            }
        }
        None => error_response("Validator not initialized"),
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
    let url_str = match from_c_string(url) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            match validator.validate_url(&url_str) {
                ValidationResult::Valid => success_with(&[("valid", serde_json::json!(true))]),
                ValidationResult::Invalid(msg) => success_with(&[
                    ("valid", serde_json::json!(false)),
                    ("error", serde_json::json!(msg)),
                ]),
                ValidationResult::Sanitized(sanitized) => success_with(&[
                    ("valid", serde_json::json!(true)),
                    ("sanitized", serde_json::json!(sanitized)),
                ]),
            }
        }
        None => error_response("Validator not initialized"),
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
    let json = match from_c_string(json_str) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            match validator.validate_json(&json) {
                Ok(_) => success_with(&[("valid", serde_json::json!(true))]),
                Err(e) => success_with(&[
                    ("valid", serde_json::json!(false)),
                    ("error", serde_json::json!(e)),
                ]),
            }
        }
        None => error_response("Validator not initialized"),
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
    let html_str = match from_c_string(html) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => {
            let sanitized = validator.sanitize_html(&html_str);
            success_with(&[("sanitized", serde_json::json!(sanitized))])
        }
        None => error_response("Validator not initialized"),
    }
}

/// Get current validation configuration
///
/// # Returns
/// JSON with current validation configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_validation_config() -> *mut c_char {
    let guard = VALIDATOR.lock().unwrap();
    match guard.as_ref() {
        Some(validator) => serialize_result(validator.config()),
        None => error_response("Validator not initialized"),
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
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let config: antikythera_core::security::config::ValidationConfig =
        match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
        };

    let mut guard = VALIDATOR.lock().unwrap();
    match guard.as_mut() {
        Some(validator) => {
            match validator.update_config(config) {
                Ok(_) => success_response(),
                Err(e) => error_response(&e),
            }
        }
        None => error_response("Validator not initialized"),
    }
}