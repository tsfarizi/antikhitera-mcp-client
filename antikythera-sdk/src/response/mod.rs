//! Response Formatting Feature Slice
//!
//! Provides output format configuration and response formatting via FFI.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, LazyLock};

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON format (structured)
    Json,
    /// Markdown format (text with formatting)
    Markdown,
    /// Plain text format
    Text,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Json
    }
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Markdown => "markdown",
            OutputFormat::Text => "text",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "text" | "plain" => Ok(OutputFormat::Text),
            _ => Err(format!("Invalid output format: {}. Use 'json', 'markdown', or 'text'", s)),
        }
    }
}

/// Server output format registry
static OUTPUT_FORMATS: LazyLock<Mutex<HashMap<u32, OutputFormat>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
#[unsafe(no_mangle)]
pub extern "C" fn mcp_set_output_format(server_id: u32, format: *const c_char) -> i32 {
    let format_str = match from_c_string(format) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Invalid format string: {}", e);
            return 0;
        }
    };

    let output_format = match OutputFormat::from_str(&format_str) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return 0;
        }
    };

    match OUTPUT_FORMATS.lock() {
        Ok(mut formats) => {
            formats.insert(server_id, output_format);
            1
        }
        Err(_) => 0,
    }
}

/// Get the current output format for a server
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_output_format(server_id: u32) -> *mut c_char {
    match OUTPUT_FORMATS.lock() {
        Ok(formats) => {
            let default_format = OutputFormat::default();
            let format = formats.get(&server_id).unwrap_or(&default_format);
            to_c_string(format.as_str())
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Format a response according to the server's output format setting
#[unsafe(no_mangle)]
pub extern "C" fn mcp_format_response(
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

    let format = OUTPUT_FORMATS.lock()
        .ok()
        .and_then(|formats| formats.get(&server_id).copied())
        .unwrap_or_default();

    let formatted = match format {
        OutputFormat::Json => {
            let mut obj = serde_json::Map::new();
            obj.insert("content".to_string(), serde_json::Value::String(content_str));
            if let Some(data) = data_value {
                obj.insert("data".to_string(), data);
            }
            obj.insert("format".to_string(), serde_json::Value::String("json".to_string()));
            serde_json::Value::Object(obj).to_string()
        }
        OutputFormat::Markdown => {
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
        }
        OutputFormat::Text => {
            let mut text = content_str.clone();
            if let Some(data) = data_value {
                text.push_str("\n\nData:\n");
                text.push_str(&data.to_string());
            }
            text
        }
    };

    to_c_string(&formatted)
}
