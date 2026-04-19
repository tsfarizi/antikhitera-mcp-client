//! JSON Schema FFI
//!
//! Expose JSON schema definition, validation, and schema management via FFI.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

use super::types::{JsonSchema, ValidationError};
use super::validator::{JsonValidator, RetryManager};

// ============================================================================
// Schema Registry
// ============================================================================

/// Global schema registry
static SCHEMA_REGISTRY: LazyLock<Mutex<HashMap<String, JsonSchema>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Global retry managers
static RETRY_MANAGERS: LazyLock<Mutex<HashMap<String, RetryManager>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// Helpers
// ============================================================================

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

fn serialize_result<T: serde::Serialize>(result: &T) -> *mut c_char {
    match serde_json::to_string(result) {
        Ok(json) => to_c_string(&json),
        Err(e) => {
            let error = serde_json::json!({"error": format!("Serialization failed: {}", e)});
            to_c_string(&error.to_string())
        }
    }
}

fn error_response(message: &str) -> *mut c_char {
    to_c_string(&format!(r#"{{"error": "{}"}}"#, message))
}

fn success_with(fields: &[(&str, serde_json::Value)]) -> *mut c_char {
    let mut obj = serde_json::Map::new();
    obj.insert("success".to_string(), serde_json::json!(true));
    for (key, value) in fields {
        obj.insert(key.to_string(), value.clone());
    }
    serialize_result(&serde_json::Value::Object(obj))
}

// ============================================================================
// Schema Management FFI
// ============================================================================

/// Register a new JSON schema from JSON definition
///
/// # Parameters
/// - `schema_name`: Unique name for this schema
/// - `schema_json`: JSON schema definition
///
/// # Returns
/// JSON with `success`, `schema_name`, and `prompt_instruction` fields
pub fn mcp_json_schema_register(
    schema_name: *const c_char,
    schema_json: *const c_char,
) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let json_str = match from_c_string(schema_json) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Parse schema
    let schema: JsonSchema = match serde_json::from_str(&json_str) {
        Ok(s) => s,
        Err(e) => return error_response(&format!("Invalid schema JSON: {}", e)),
    };

    let prompt_instruction = schema.to_prompt_instruction();

    // Register
    let mut registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    registry.insert(name_str.clone(), schema);

    success_with(&[
        ("schema_name", serde_json::json!(name_str)),
        ("prompt_instruction", serde_json::json!(prompt_instruction)),
    ])
}

/// Get registered schema by name
///
/// # Returns
/// JSON schema definition or error
pub fn mcp_json_schema_get(schema_name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    match registry.get(&name_str) {
        Some(schema) => serialize_result(schema),
        None => error_response(&format!("Schema '{}' not found", name_str)),
    }
}

/// List all registered schemas
///
/// # Returns
/// JSON array of schema names
pub fn mcp_json_schema_list() -> *mut c_char {
    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    let names: Vec<&str> = registry.keys().map(|s| s.as_str()).collect();
    serialize_result(&names)
}

/// Remove a schema by name
pub fn mcp_json_schema_remove(schema_name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    if registry.remove(&name_str).is_some() {
        success_with(&[("schema_name", serde_json::json!(name_str))])
    } else {
        error_response(&format!("Schema '{}' not found", name_str))
    }
}

/// Generate example JSON from a registered schema
pub fn mcp_json_schema_example(schema_name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    match registry.get(&name_str) {
        Some(schema) => to_c_string(&schema.generate_example()),
        None => error_response(&format!("Schema '{}' not found", name_str)),
    }
}

// ============================================================================
// Validation FFI
// ============================================================================

/// Validate JSON response against a registered schema
///
/// # Parameters
/// - `schema_name`: Registered schema name
/// - `json_response`: JSON response to validate
/// - `max_retries`: Maximum retry attempts (0 for no retries)
///
/// # Returns
/// JSON with `valid`, `error`, `retry_count`, and `json` fields
pub fn mcp_json_validate(
    schema_name: *const c_char,
    json_response: *const c_char,
    max_retries: u32,
) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let json_str = match from_c_string(json_response) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    match registry.get(&name_str) {
        Some(schema) => {
            let validator = JsonValidator::new(schema.clone()).with_max_retries(max_retries);
            let result = validator.validate_with_retry(&json_str);
            serialize_result(&result)
        }
        None => error_response(&format!("Schema '{}' not found", name_str)),
    }
}

/// Get schema prompt instruction to append to LLM prompt
pub fn mcp_json_schema_prompt(schema_name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    match registry.get(&name_str) {
        Some(schema) => to_c_string(&schema.to_prompt_instruction()),
        None => error_response(&format!("Schema '{}' not found", name_str)),
    }
}

// ============================================================================
// Retry Management FFI
// ============================================================================

/// Initialize retry manager for a session
///
/// # Parameters
/// - `session_id`: Unique session identifier
/// - `max_retries`: Maximum retry attempts
pub fn mcp_json_retry_init(session_id: *const c_char, max_retries: u32) -> *mut c_char {
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut managers = match RETRY_MANAGERS.lock() {
        Ok(m) => m,
        Err(e) => return error_response(&format!("Failed to lock managers: {}", e)),
    };

    managers.insert(session_str.clone(), RetryManager::new(max_retries));

    success_with(&[
        ("session_id", serde_json::json!(session_str)),
        ("max_retries", serde_json::json!(max_retries)),
    ])
}

/// Record a validation error for retry tracking
pub fn mcp_json_retry_record_error(
    session_id: *const c_char,
    error_message: *const c_char,
) -> *mut c_char {
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let error_str = match from_c_string(error_message) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let mut managers = match RETRY_MANAGERS.lock() {
        Ok(m) => m,
        Err(e) => return error_response(&format!("Failed to lock managers: {}", e)),
    };

    match managers.get_mut(&session_str) {
        Some(manager) => {
            let error = ValidationError::InvalidJson(error_str);
            manager.record_error(&error);

            success_with(&[
                ("session_id", serde_json::json!(session_str)),
                ("attempt", serde_json::json!(manager.current_attempt)),
                ("exhausted", serde_json::json!(manager.is_exhausted())),
            ])
        }
        None => error_response(&format!(
            "Retry manager not found for session '{}'",
            session_str
        )),
    }
}

/// Generate retry prompt for LLM
pub fn mcp_json_retry_prompt(
    session_id: *const c_char,
    schema_name: *const c_char,
    last_response: *const c_char,
) -> *mut c_char {
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let name_str = match from_c_string(schema_name) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let last_str = match from_c_string(last_response) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let managers = match RETRY_MANAGERS.lock() {
        Ok(m) => m,
        Err(e) => return error_response(&format!("Failed to lock managers: {}", e)),
    };

    let registry = match SCHEMA_REGISTRY.lock() {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Failed to lock registry: {}", e)),
    };

    match (managers.get(&session_str), registry.get(&name_str)) {
        (Some(manager), Some(schema)) => {
            let schema_prompt = schema.to_prompt_instruction();
            let retry_prompt = manager.retry_prompt(&schema_prompt, &last_str);
            to_c_string(&retry_prompt)
        }
        (None, _) => error_response(&format!(
            "Retry manager not found for session '{}'",
            session_str
        )),
        (_, None) => error_response(&format!("Schema '{}' not found", name_str)),
    }
}

/// Check if retries are exhausted for a session
pub fn mcp_json_retry_is_exhausted(session_id: *const c_char) -> *mut c_char {
    let session_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let managers = match RETRY_MANAGERS.lock() {
        Ok(m) => m,
        Err(e) => return error_response(&format!("Failed to lock managers: {}", e)),
    };

    match managers.get(&session_str) {
        Some(manager) => success_with(&[
            ("session_id", serde_json::json!(session_str)),
            ("exhausted", serde_json::json!(manager.is_exhausted())),
            ("attempts", serde_json::json!(manager.current_attempt)),
        ]),
        None => error_response(&format!(
            "Retry manager not found for session '{}'",
            session_str
        )),
    }
}
