//! Common FFI utilities for security module

use std::os::raw::c_char;
use std::ffi::{CStr, CString};

/// Convert C string to Rust string
pub fn from_c_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Null pointer provided".to_string());
    }

    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

/// Convert Rust string to C string (caller must free)
pub fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create success response
pub fn success_response() -> *mut c_char {
    let json = serde_json::json!({
        "success": true
    });
    serialize_result(&json)
}

/// Create success response with additional fields
pub fn success_with(fields: &[(&str, serde_json::Value)]) -> *mut c_char {
    let mut json = serde_json::json!({
        "success": true
    });

    if let Some(obj) = json.as_object_mut() {
        for (key, value) in fields {
            obj.insert(key.to_string(), value.clone());
        }
    }

    serialize_result(&json)
}

/// Create error response
pub fn error_response(message: &str) -> *mut c_char {
    let json = serde_json::json!({
        "success": false,
        "error": message
    });
    serialize_result(&json)
}

/// Serialize result to C string
pub fn serialize_result<T: serde::Serialize>(value: &T) -> *mut c_char {
    match serde_json::to_string(value) {
        Ok(json) => to_c_string(&json),
        Err(_) => error_response("Failed to serialize result"),
    }
}

/// Free C string allocated by Rust
#[unsafe(no_mangle)]
pub extern "C" fn mcp_security_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
