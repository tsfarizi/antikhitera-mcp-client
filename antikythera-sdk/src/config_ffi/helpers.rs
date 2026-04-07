//! FFI Helper Functions
//!
//! Common utilities used across all config FFI modules.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Convert Rust string to C string (caller must free with `mcp_string_free`)
pub fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Convert C string to Rust string
pub fn from_c_string(ptr: *const c_char) -> Result<String, String> {
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

/// Serialize any serializable value to JSON C string
pub fn serialize_result<T: serde::Serialize>(result: &T) -> *mut c_char {
    match serde_json::to_string(result) {
        Ok(json) => to_c_string(&json),
        Err(e) => {
            let error = serde_json::json!({"error": format!("Serialization failed: {}", e)});
            to_c_string(&error.to_string())
        }
    }
}

/// Create error response C string
pub fn error_response(message: &str) -> *mut c_char {
    to_c_string(&format!(r#"{{"error": "{}"}}"#, message))
}

/// Create success response C string
pub fn success_response() -> *mut c_char {
    to_c_string(r#"{"success": true}"#)
}

/// Create success response with additional fields
pub fn success_with(fields: &[(&str, serde_json::Value)]) -> *mut c_char {
    let mut obj = serde_json::Map::new();
    obj.insert("success".to_string(), serde_json::json!(true));
    for (key, value) in fields {
        obj.insert(key.to_string(), value.clone());
    }
    serialize_result(&serde_json::Value::Object(obj))
}
