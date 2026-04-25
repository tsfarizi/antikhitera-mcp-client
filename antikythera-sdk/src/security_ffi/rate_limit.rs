//! Rate Limiting FFI

use super::helpers::*;
use crate::sdk_logging::SecurityFfiLogger;
use antikythera_core::security::rate_limit::RateLimiter;
use std::os::raw::c_char;
use std::sync::Mutex;

/// Global rate limiter instance (thread-safe)
static RATE_LIMITER: Mutex<Option<RateLimiter>> = Mutex::new(None);

/// Initialize the rate limiter with default configuration
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_init_rate_limiter() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_init_rate_limiter", "{}");

    let limiter = RateLimiter::from_config();
    let mut guard = RATE_LIMITER.lock().unwrap();
    *guard = Some(limiter);

    logger.ffi_result("mcp_security_init_rate_limiter", true, 0);
    success_response()
}

/// Check if a request is allowed for a session
///
/// # Parameters
/// - `session_id`: Session identifier
///
/// # Returns
/// JSON with `success`, `allowed`, and optional `error` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_check_rate_limit(session_id: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_check_rate_limit", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_check_rate_limit",
        &format!("{{\"session_id\": \"{}\"}}", session_str),
    );

    let guard = RATE_LIMITER.lock().unwrap();
    match guard.as_ref() {
        Some(limiter) => match limiter.check(&session_str) {
            Ok(_) => {
                logger.rate_limit_checked(&session_str, true);
                logger.ffi_result("mcp_security_check_rate_limit", true, 0);
                success_with(&[("allowed", serde_json::json!(true))])
            }
            Err(e) => {
                let err_msg = e.to_string();
                logger.rate_limit_exceeded(&session_str, &err_msg);
                logger.ffi_result("mcp_security_check_rate_limit", false, 0);
                success_with(&[
                    ("allowed", serde_json::json!(false)),
                    ("error", serde_json::json!(err_msg)),
                ])
            }
        },
        None => {
            logger.ffi_error(
                "mcp_security_check_rate_limit",
                "Rate limiter not initialized",
            );
            error_response("Rate limiter not initialized")
        }
    }
}

/// Get usage statistics for a session
///
/// # Parameters
/// - `session_id`: Session identifier
///
/// # Returns
/// JSON with usage statistics
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_usage(session_id: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_get_usage", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_get_usage",
        &format!("{{\"session_id\": \"{}\"}}", session_str),
    );

    let guard = RATE_LIMITER.lock().unwrap();
    match guard.as_ref() {
        Some(limiter) => match limiter.get_usage(&session_str) {
            Some(usage) => {
                logger.ffi_result("mcp_security_get_usage", true, 0);
                let json = serde_json::json!({
                    "success": true,
                    "requests_per_minute": usage.requests_per_minute,
                    "requests_per_hour": usage.requests_per_hour,
                    "requests_per_day": usage.requests_per_day,
                    "last_activity": usage.last_activity.elapsed().as_secs()
                });
                serialize_result(&json)
            }
            None => {
                logger.ffi_error("mcp_security_get_usage", "Session not found");
                error_response("Session not found")
            }
        },
        None => {
            logger.ffi_error("mcp_security_get_usage", "Rate limiter not initialized");
            error_response("Rate limiter not initialized")
        }
    }
}

/// Reset rate limits for a session
///
/// # Parameters
/// - `session_id`: Session identifier
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_reset_session(session_id: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_reset_session", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_reset_session",
        &format!("{{\"session_id\": \"{}\"}}", session_str),
    );

    let guard = RATE_LIMITER.lock().unwrap();
    match guard.as_ref() {
        Some(limiter) => {
            limiter.reset_session(&session_str);
            logger.ffi_result("mcp_security_reset_session", true, 0);
            success_response()
        }
        None => {
            logger.ffi_error("mcp_security_reset_session", "Rate limiter not initialized");
            error_response("Rate limiter not initialized")
        }
    }
}

/// Remove a session
///
/// # Parameters
/// - `session_id`: Session identifier
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_remove_session(session_id: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_remove_session", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call(
        "mcp_security_remove_session",
        &format!("{{\"session_id\": \"{}\"}}", session_str),
    );

    let guard = RATE_LIMITER.lock().unwrap();
    match guard.as_ref() {
        Some(limiter) => {
            limiter.remove_session(&session_str);
            logger.ffi_result("mcp_security_remove_session", true, 0);
            success_response()
        }
        None => {
            logger.ffi_error(
                "mcp_security_remove_session",
                "Rate limiter not initialized",
            );
            error_response("Rate limiter not initialized")
        }
    }
}

/// Get current rate limit configuration
///
/// # Returns
/// JSON with current rate limit configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_get_rate_limit_config() -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    logger.ffi_call("mcp_security_get_rate_limit_config", "{}");

    let guard = RATE_LIMITER.lock().unwrap();
    match guard.as_ref() {
        Some(limiter) => {
            logger.ffi_result("mcp_security_get_rate_limit_config", true, 0);
            serialize_result(limiter.config())
        }
        None => {
            logger.ffi_error(
                "mcp_security_get_rate_limit_config",
                "Rate limiter not initialized",
            );
            error_response("Rate limiter not initialized")
        }
    }
}

/// Set rate limit configuration
///
/// # Parameters
/// - `config_json`: Rate limit configuration as JSON string
///
/// # Returns
/// `{"success": true}` or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_set_rate_limit_config(config_json: *const c_char) -> *mut c_char {
    let logger = SecurityFfiLogger::new("security");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            logger.ffi_error("mcp_security_set_rate_limit_config", &e);
            return error_response(&e);
        }
    };

    logger.ffi_call("mcp_security_set_rate_limit_config", &json_str);

    let config: antikythera_core::security::config::RateLimitConfig =
        match serde_json::from_str(&json_str) {
            Ok(c) => c,
            Err(e) => {
                let err = format!("Invalid JSON: {}", e);
                logger.ffi_error("mcp_security_set_rate_limit_config", &err);
                return error_response(&err);
            }
        };

    let mut guard = RATE_LIMITER.lock().unwrap();
    match guard.as_mut() {
        Some(limiter) => {
            limiter.update_config(config);
            logger.ffi_result("mcp_security_set_rate_limit_config", true, 0);
            success_response()
        }
        None => {
            logger.ffi_error(
                "mcp_security_set_rate_limit_config",
                "Rate limiter not initialized",
            );
            error_response("Rate limiter not initialized")
        }
    }
}
