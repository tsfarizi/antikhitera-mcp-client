//! Response Formatting Feature Slice
//!
//! Provides output format configuration and response formatting via FFI.
//! When `format_is_json` is true, responses are formatted as JSON.
//! When false, responses are formatted as Markdown/Text.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, LazyLock};

/// Server output format registry (true = JSON, false = Markdown/Text)
static OUTPUT_FORMATS: LazyLock<Mutex<HashMap<u32, bool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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

/// Set the output format for server responses
///
/// # Parameters
/// - `server_id`: Server ID
/// - `format_is_json`: true for JSON format, false for Markdown/Text format
pub fn mcp_set_output_format(server_id: u32, format_is_json: i32) -> i32 {
    match OUTPUT_FORMATS.lock() {
        Ok(mut formats) => {
            formats.insert(server_id, format_is_json != 0);
            1
        }
        Err(_) => 0,
    }
}

/// Get the current output format for a server
///
/// # Returns
/// "true" if JSON format, "false" if Markdown/Text format
pub fn mcp_get_output_format(server_id: u32) -> *mut c_char {
    match OUTPUT_FORMATS.lock() {
        Ok(formats) => {
            let is_json = formats.get(&server_id).copied().unwrap_or(true);
            to_c_string(if is_json { "true" } else { "false" })
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Format a response according to the server's output format setting
///
/// # Parameters
/// - `server_id`: Server ID
/// - `content`: Response content
/// - `data_json`: Optional data as JSON (can be NULL)
///
/// # Returns
/// Formatted response string
pub fn mcp_format_response(
    server_id: u32,
    content: *const c_char,
    data_json: *const c_char,
) -> *mut c_char {
    let content_str = match from_c_string(content) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    let data_value = if data_json.is_null() {
        None
    } else {
        match from_c_string(data_json) {
            Ok(s) => serde_json::from_str::<serde_json::Value>(&s).ok(),
            Err(_) => None,
        }
    };

    // Get format setting (true = JSON, false = Markdown/Text)
    let format_is_json = OUTPUT_FORMATS.lock()
        .ok()
        .and_then(|formats| formats.get(&server_id).copied())
        .unwrap_or(true); // Default to JSON

    let formatted = if format_is_json {
        // JSON format
        let mut obj = serde_json::Map::new();
        obj.insert("content".to_string(), serde_json::Value::String(content_str));
        if let Some(data) = data_value {
            obj.insert("data".to_string(), data);
        }
        obj.insert("format_is_json".to_string(), serde_json::Value::Bool(true));
        serde_json::Value::Object(obj).to_string()
    } else {
        // Markdown/Text format
        let mut md = String::new();
        md.push_str("# Response\n\n");
        md.push_str(&content_str);
        if let Some(data) = data_value {
            md.push_str("\n\n## Data\n\n");
            md.push_str("```json\n");
            md.push_str(&data.to_string());
            md.push_str("\n```\n");
        }
        md
    };

    to_c_string(&formatted)
}

