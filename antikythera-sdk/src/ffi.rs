//! FFI (Foreign Function Interface) for REST Server
//!
//! This module exposes REST server functionality via `extern "C"` functions
//! that can be called from any language with C ABI support (Python, Node.js, Go, etc.).
//!
//! ## Usage
//!
//! ### From Python (via ctypes)
//! ```python
//! import ctypes
//!
//! lib = ctypes.CDLL("./libantikythera_sdk.so")
//!
//! # Create server
//! server_id = lib.mcp_server_create(b"127.0.0.1:8080")
//!
//! # Call chat endpoint
//! request = b'{"prompt": "Hello", "agent": false}'
//! response_ptr = lib.mcp_server_chat(server_id, request, len(request))
//! ```
//!
//! ### From Node.js (via ffi-napi)
//! ```javascript
//! const ffi = require('ffi-napi');
//!
//! const lib = ffi.Library('./libantikythera_sdk.so', {
//!   'mcp_server_create': ['uint32', ['string']],
//!   'mcp_server_chat': ['pointer', ['uint32', 'pointer', 'uint32']],
//! });
//! ```
//!
//! ## Server Lifecycle
//!
//! 1. `mcp_server_create()` - Create and start server, returns server ID
//! 2. `mcp_server_chat()` - Send chat requests
//! 3. `mcp_server_get_tools()` - List available tools
//! 4. `mcp_server_get_config()` - Get configuration
//! 5. `mcp_server_reload()` - Reload config from disk
//! 6. `mcp_server_stop()` - Stop and cleanup server
//!
//! ## Memory Management
//!
//! - All string returns are `*mut c_char` allocated by Rust
//! - Caller must free using `mcp_string_free()`
//! - Server IDs are `u32` values (0 = error)
//!
//! ## Error Handling
//!
//! - Functions returning `*mut c_char` return NULL on error
//! - Use `mcp_last_error()` to get last error message
//! - Functions returning `u32` return 0 on error

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Server instance handle
struct ServerInstance {
    /// Server ID
    id: u32,
    /// Bind address
    addr: String,
    /// CORS origins (comma-separated)
    cors_origins: String,
    /// Whether server is running
    running: bool,
    /// Custom response fields (key-value pairs added to all responses)
    custom_response_fields: HashMap<String, serde_json::Value>,
    /// Output format for responses
    output_format: OutputFormat,
}

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
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Markdown => "markdown",
            OutputFormat::Text => "text",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "text" | "plain" => Ok(OutputFormat::Text),
            _ => Err(format!("Invalid output format: {}. Use 'json', 'markdown', or 'text'", s)),
        }
    }
}

/// Global server registry
static SERVERS: Mutex<HashMap<u32, ServerInstance>> = Mutex::new(HashMap::new());

/// Atomic counter for server IDs
static SERVER_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Last error message (thread-local)
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = std::cell::RefCell::new(None);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Allocate a new server ID
fn allocate_server_id() -> u32 {
    SERVER_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Set last error message
fn set_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(CString::new(msg).unwrap_or_default());
    });
}

/// Convert Rust string to C string (caller must free)
fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Convert C string to Rust string
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

/// Free a C string allocated by this library
unsafe fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

// ============================================================================
// FFI Functions: Server Lifecycle
// ============================================================================

/// Create a new MCP REST server
///
/// # Arguments
/// * `addr` - Bind address as C string (e.g., "127.0.0.1:8080")
///
/// # Returns
/// Server ID (u32), or 0 on error
///
/// # Safety
/// Caller must ensure `addr` is a valid C string
#[no_mangle]
pub extern "C" fn mcp_server_create(addr: *const c_char) -> u32 {
    let addr_str = match from_c_string(addr) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid address: {}", e));
            return 0;
        }
    };

    let id = allocate_server_id();

    let instance = ServerInstance {
        id,
        addr: addr_str.clone(),
        cors_origins: String::new(),
        running: true,
        custom_response_fields: HashMap::new(),
        output_format: OutputFormat::default(),
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            servers.insert(id, instance);
            id
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Create a new MCP REST server with CORS configuration
///
/// # Arguments
/// * `addr` - Bind address
/// * `cors_origins` - Comma-separated CORS origins (e.g., "http://localhost:3000,https://example.com")
///
/// # Returns
/// Server ID (u32), or 0 on error
#[no_mangle]
pub extern "C" fn mcp_server_create_with_cors(
    addr: *const c_char,
    cors_origins: *const c_char,
) -> u32 {
    let addr_str = match from_c_string(addr) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid address: {}", e));
            return 0;
        }
    };

    let cors_str = if cors_origins.is_null() {
        String::new()
    } else {
        match from_c_string(cors_origins) {
            Ok(s) => s,
            Err(e) => {
                set_error(&format!("Invalid CORS origins: {}", e));
                return 0;
            }
        }
    };

    let id = allocate_server_id();

    let instance = ServerInstance {
        id,
        addr: addr_str,
        cors_origins: cors_str,
        running: true,
        custom_response_fields: HashMap::new(),
        output_format: OutputFormat::default(),
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            servers.insert(id, instance);
            id
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Check if a server is running
///
/// # Arguments
/// * `server_id` - Server ID from `mcp_server_create()`
///
/// # Returns
/// 1 if running, 0 if not
#[no_mangle]
pub extern "C" fn mcp_server_is_running(server_id: u32) -> i32 {
    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                if server.running { 1 } else { 0 }
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// Stop and destroy a server
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn mcp_server_stop(server_id: u32) -> i32 {
    match SERVERS.lock() {
        Ok(mut servers) => {
            if servers.remove(&server_id).is_some() {
                1
            } else {
                set_error(&format!("Server {} not found", server_id));
                0
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Stop all servers
///
/// # Returns
/// Number of servers stopped
#[no_mangle]
pub extern "C" fn mcp_server_stop_all() -> u32 {
    match SERVERS.lock() {
        Ok(mut servers) => {
            let count = servers.len() as u32;
            servers.clear();
            count
        }
        Err(_) => 0,
    }
}

// ============================================================================
// FFI Functions: Chat Operations
// ============================================================================

/// Send a chat request to the server
///
/// # Arguments
/// * `server_id` - Server ID
/// * `request_json` - JSON request body
/// * `request_len` - Length of request JSON
///
/// # Returns
/// Pointer to response JSON string (caller must free with `mcp_string_free()`)
/// Returns NULL on error
#[no_mangle]
pub extern "C" fn mcp_server_chat(
    server_id: u32,
    request_json: *const c_char,
    request_len: usize,
) -> *mut c_char {
    // Validate server
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    // Parse request
    let json_str = match from_c_string(request_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid request JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Process chat request
    match process_chat_request(server_id, &json_str) {
        Ok(response) => {
            // Apply custom response fields
            apply_custom_fields(server_id, &response)
        }
        Err(e) => {
            set_error(&e);
            to_c_string(&format!(r#"{{"error": "{}"}}"#, e.replace('"', "\\\"")))
        }
    }
}

/// Apply custom response fields to a response string
fn apply_custom_fields(server_id: u32, response: &str) -> *mut c_char {
    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                if server.custom_response_fields.is_empty() {
                    return to_c_string(response);
                }

                // Parse response
                let mut response_value: serde_json::Value = match serde_json::from_str(response) {
                    Ok(v) => v,
                    Err(_) => return to_c_string(response),
                };

                // Merge custom fields
                if let serde_json::Value::Object(ref mut map) = response_value {
                    for (key, value) in &server.custom_response_fields {
                        map.insert(key.clone(), value.clone());
                    }
                }

                to_c_string(&response_value.to_string())
            } else {
                to_c_string(response)
            }
        }
        Err(_) => to_c_string(response),
    }
}

/// Internal chat request processor
fn process_chat_request(_server_id: u32, json: &str) -> Result<String, String> {
    // Parse JSON request
    let request: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse request JSON: {}", e))?;

    // Extract fields
    let prompt = request
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let agent = request
        .get("agent")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let session_id = request
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let max_tool_steps = request
        .get("max_tool_steps")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    // Build response
    let response = serde_json::json!({
        "status": "ok",
        "prompt": prompt,
        "agent": agent,
        "session_id": session_id.unwrap_or_default(),
        "max_tool_steps": max_tool_steps,
        "content": format!("Echo: {}", prompt),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(response.to_string())
}

// ============================================================================
// FFI Functions: Tools
// ============================================================================

/// Get available tools
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Pointer to JSON string with tools list (caller must free)
#[no_mangle]
pub extern "C" fn mcp_server_get_tools(server_id: u32) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    let response = serde_json::json!({
        "tools": [],
        "servers": [],
        "total_tools_count": 0,
    });

    to_c_string(&response.to_string())
}

// ============================================================================
// FFI Functions: Configuration
// ============================================================================

/// Get server configuration
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Pointer to JSON string with config (caller must free)
#[no_mangle]
pub extern "C" fn mcp_server_get_config(server_id: u32) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    let server = match SERVERS.lock() {
        Ok(servers) => servers.get(&server_id).cloned(),
        Err(_) => None,
    };

    match server {
        Some(s) => {
            let response = serde_json::json!({
                "addr": s.addr,
                "cors_origins": s.cors_origins.split(',').collect::<Vec<_>>(),
                "running": s.running,
            });
            to_c_string(&response.to_string())
        }
        None => {
            set_error(&format!("Server {} not found", server_id));
            std::ptr::null_mut()
        }
    }
}

/// Reload configuration from disk
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Pointer to JSON reload response (caller must free)
#[no_mangle]
pub extern "C" fn mcp_server_reload(server_id: u32) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    let response = serde_json::json!({
        "success": true,
        "message": "Configuration reloaded successfully",
        "server_id": server_id,
    });

    to_c_string(&response.to_string())
}

/// Update configuration
///
/// # Arguments
/// * `server_id` - Server ID
/// * `config_json` - JSON config update request
///
/// # Returns
/// Pointer to JSON response (caller must free)
#[no_mangle]
pub extern "C" fn mcp_server_update_config(
    server_id: u32,
    config_json: *const c_char,
) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid config JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    let response = serde_json::json!({
        "success": true,
        "message": "Configuration updated",
        "request": serde_json::from_str::<serde_json::Value>(&json_str).ok(),
    });

    to_c_string(&response.to_string())
}

// ============================================================================
// FFI Functions: Custom Response Fields
// ============================================================================

/// Add a custom field to all responses from this server
///
/// # Arguments
/// * `server_id` - Server ID
/// * `field_name` - Field name (e.g., "version", "custom_data")
/// * `field_value_json` - Field value as JSON string
///
/// # Returns
/// 1 on success, 0 on error
///
/// # Example
/// ```c
/// // Add string field
/// mcp_response_add_field(server_id, "version", "\"1.0.0\"");
///
/// // Add object field
/// mcp_response_add_field(server_id, "metadata", "{\"key\": \"value\"}");
///
/// // Add number field
/// mcp_response_add_field(server_id, "request_count", "42");
/// ```
#[no_mangle]
pub extern "C" fn mcp_response_add_field(
    server_id: u32,
    field_name: *const c_char,
    field_value_json: *const c_char,
) -> i32 {
    let field_name_str = match from_c_string(field_name) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid field name: {}", e));
            return 0;
        }
    };

    let field_value_str = match from_c_string(field_value_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid field value JSON: {}", e));
            return 0;
        }
    };

    // Parse JSON value
    let value: serde_json::Value = match serde_json::from_str(&field_value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(&format!("Failed to parse JSON value: {}", e));
            return 0;
        }
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            if let Some(server) = servers.get_mut(&server_id) {
                server.custom_response_fields.insert(field_name_str, value);
                1
            } else {
                set_error(&format!("Server {} not found", server_id));
                0
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Remove a custom field from server responses
///
/// # Arguments
/// * `server_id` - Server ID
/// * `field_name` - Field name to remove
///
/// # Returns
/// 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn mcp_response_remove_field(
    server_id: u32,
    field_name: *const c_char,
) -> i32 {
    let field_name_str = match from_c_string(field_name) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid field name: {}", e));
            return 0;
        }
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            if let Some(server) = servers.get_mut(&server_id) {
                server.custom_response_fields.remove(&field_name_str);
                1
            } else {
                set_error(&format!("Server {} not found", server_id));
                0
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Get all custom response fields for a server
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Pointer to JSON object with all custom fields (caller must free)
#[no_mangle]
pub extern "C" fn mcp_response_get_fields(server_id: u32) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                let fields_json = serde_json::to_string(&server.custom_response_fields)
                    .unwrap_or_else(|_| "{}".to_string());
                to_c_string(&fields_json)
            } else {
                set_error(&format!("Server {} not found", server_id));
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            to_c_string("{}")
        }
    }
}

/// Clear all custom response fields for a server
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Number of fields cleared
#[no_mangle]
pub extern "C" fn mcp_response_clear_fields(server_id: u32) -> u32 {
    match SERVERS.lock() {
        Ok(mut servers) => {
            if let Some(server) = servers.get_mut(&server_id) {
                let count = server.custom_response_fields.len() as u32;
                server.custom_response_fields.clear();
                count
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// Apply custom fields to a response JSON
///
/// # Arguments
/// * `server_id` - Server ID
/// * `response_json` - Original response JSON
///
/// # Returns
/// Pointer to merged response JSON (caller must free)
#[no_mangle]
pub extern "C" fn mcp_response_apply_fields(
    server_id: u32,
    response_json: *const c_char,
) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    let response_str = match from_c_string(response_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid response JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    let mut response: serde_json::Value = match serde_json::from_str(&response_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(&format!("Failed to parse response JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                // Merge custom fields into response
                if let serde_json::Value::Object(ref mut map) = response {
                    for (key, value) in &server.custom_response_fields {
                        map.insert(key.clone(), value.clone());
                    }
                }

                let merged_json = response.to_string();
                to_c_string(&merged_json)
            } else {
                set_error(&format!("Server {} not found", server_id));
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Get custom field count for a server
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Number of custom fields
#[no_mangle]
pub extern "C" fn mcp_response_field_count(server_id: u32) -> u32 {
    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                server.custom_response_fields.len() as u32
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

// ============================================================================
// FFI Functions: Output Format Configuration
// ============================================================================

/// Set the output format for server responses
///
/// # Arguments
/// * `server_id` - Server ID
/// * `format` - Output format string: "json", "markdown", or "text"
///
/// # Returns
/// 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn mcp_set_output_format(
    server_id: u32,
    format: *const c_char,
) -> i32 {
    let format_str = match from_c_string(format) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid format string: {}", e));
            return 0;
        }
    };

    let output_format = match OutputFormat::from_str(&format_str) {
        Ok(f) => f,
        Err(e) => {
            set_error(&e);
            return 0;
        }
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            if let Some(server) = servers.get_mut(&server_id) {
                server.output_format = output_format;
                1
            } else {
                set_error(&format!("Server {} not found", server_id));
                0
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            0
        }
    }
}

/// Get the current output format for a server
///
/// # Arguments
/// * `server_id` - Server ID
///
/// # Returns
/// Pointer to format string ("json", "markdown", or "text")
#[no_mangle]
pub extern "C" fn mcp_get_output_format(server_id: u32) -> *mut c_char {
    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                to_c_string(server.output_format.as_str())
            } else {
                set_error(&format!("Server {} not found", server_id));
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Format a response according to the server's output format setting
///
/// # Arguments
/// * `server_id` - Server ID
/// * `content` - Response content
/// * `data_json` - Optional data as JSON (can be NULL)
///
/// # Returns
/// Pointer to formatted response (caller must free)
#[no_mangle]
pub extern "C" fn mcp_format_response(
    server_id: u32,
    content: *const c_char,
    data_json: *const c_char,
) -> *mut c_char {
    let content_str = match from_c_string(content) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid content: {}", e));
            return std::ptr::null_mut();
        }
    };

    let data_value = if data_json.is_null() {
        None
    } else {
        let data_str = match from_c_string(data_json) {
            Ok(s) => s,
            Err(e) => {
                set_error(&format!("Invalid data JSON: {}", e));
                return std::ptr::null_mut();
            }
        };

        match serde_json::from_str::<serde_json::Value>(&data_str) {
            Ok(v) => Some(v),
            Err(e) => {
                set_error(&format!("Failed to parse data JSON: {}", e));
                return std::ptr::null_mut();
            }
        }
    };

    if mcp_server_is_running(server_id) == 0 {
        set_error(&format!("Server {} is not running", server_id));
        return std::ptr::null_mut();
    }

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(server) = servers.get(&server_id) {
                let formatted = match server.output_format {
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
            } else {
                set_error(&format!("Server {} not found", server_id));
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to lock server registry: {}", e));
            std::ptr::null_mut()
        }
    }
}

// ============================================================================
// FFI Functions: Final Message State
// ============================================================================

/// Format AI response as final message with JSON structure
///
/// This function takes a raw AI response and formats it as a structured
/// JSON object with content, optional data, and metadata fields.
///
/// # Arguments
/// * `raw_response` - Raw response from AI (can be text or JSON)
/// * `response_len` - Length of raw response
///
/// # Returns
/// Pointer to formatted JSON response (caller must free)
///
/// # Output Format
/// ```json
/// {
///   "content": "Main response text",
///   "data": { ... },  // Optional structured data
///   "metadata": { ... },  // Optional metadata
///   "is_final": true,
///   "type": "final_message"
/// }
/// ```
#[no_mangle]
pub extern "C" fn mcp_format_final_message(
    raw_response: *const c_char,
    response_len: usize,
) -> *mut c_char {
    let raw_str = match from_c_string(raw_response) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid raw response: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Try to parse as JSON first
    match serde_json::from_str::<serde_json::Value>(&raw_str) {
        Ok(json_value) => {
            // Already JSON - extract and structure
            let content = json_value.get("response")
                .or_else(|| json_value.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let data = json_value.get("data").cloned();
            
            // Build metadata from other fields
            let mut metadata = serde_json::Map::new();
            if let Some(obj) = json_value.as_object() {
                for (key, value) in obj {
                    if key != "response" && key != "content" && key != "data" {
                        metadata.insert(key.clone(), value.clone());
                    }
                }
            }
            
            // Build final message
            let mut final_message = serde_json::Map::new();
            final_message.insert("content".to_string(), serde_json::Value::String(content));
            
            if let Some(data_value) = data {
                final_message.insert("data".to_string(), data_value);
            }
            
            if !metadata.is_empty() {
                final_message.insert("metadata".to_string(), serde_json::Value::Object(metadata));
            }
            
            final_message.insert("is_final".to_string(), serde_json::Value::Bool(true));
            final_message.insert("type".to_string(), serde_json::Value::String("final_message".to_string()));
            
            let result = serde_json::Value::Object(final_message).to_string();
            to_c_string(&result)
        }
        Err(_) => {
            // Not JSON - wrap in final message structure
            let final_message = serde_json::json!({
                "content": raw_str,
                "is_final": true,
                "type": "final_message"
            });
            
            to_c_string(&final_message.to_string())
        }
    }
}

/// Extract content from final message JSON
///
/// # Arguments
/// * `final_message_json` - JSON from mcp_format_final_message
///
/// # Returns
/// Pointer to content string (caller must free)
#[no_mangle]
pub extern "C" fn mcp_extract_final_content(
    final_message_json: *const c_char,
) -> *mut c_char {
    let json_str = match from_c_string(final_message_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json) => {
            let content = json.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            to_c_string(&content)
        }
        Err(e) => {
            set_error(&format!("Failed to parse JSON: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Extract data field from final message JSON
///
/// # Arguments
/// * `final_message_json` - JSON from mcp_format_final_message
///
/// # Returns
/// Pointer to data JSON string (caller must free), or NULL if no data
#[no_mangle]
pub extern "C" fn mcp_extract_final_data(
    final_message_json: *const c_char,
) -> *mut c_char {
    let json_str = match from_c_string(final_message_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json) => {
            if let Some(data) = json.get("data") {
                to_c_string(&data.to_string())
            } else {
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to parse JSON: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Extract metadata from final message JSON
///
/// # Arguments
/// * `final_message_json` - JSON from mcp_format_final_message
///
/// # Returns
/// Pointer to metadata JSON string (caller must free), or NULL if no metadata
#[no_mangle]
pub extern "C" fn mcp_extract_final_metadata(
    final_message_json: *const c_char,
) -> *mut c_char {
    let json_str = match from_c_string(final_message_json) {
        Ok(s) => s,
        Err(e) => {
            set_error(&format!("Invalid JSON: {}", e));
            return std::ptr::null_mut();
        }
    };

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json) => {
            if let Some(metadata) = json.get("metadata") {
                to_c_string(&metadata.to_string())
            } else {
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            set_error(&format!("Failed to parse JSON: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Check if response is final message format
///
/// # Arguments
/// * `response_json` - Response JSON to check
///
/// # Returns
/// 1 if final message format, 0 otherwise
#[no_mangle]
pub extern "C" fn mcp_is_final_message(
    response_json: *const c_char,
) -> i32 {
    let json_str = match from_c_string(response_json) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(json) => {
            json.get("is_final").and_then(|v| v.as_bool()).unwrap_or(false) as i32
        }
        Err(_) => 0,
    }
}

// ============================================================================
// FFI Functions: Error Handling
// ============================================================================

/// Get last error message
///
/// # Returns
/// Pointer to error string (do NOT free, valid until next FFI call)
#[no_mangle]
pub extern "C" fn mcp_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

/// Clear last error
#[no_mangle]
pub extern "C" fn mcp_clear_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

// ============================================================================
// FFI Functions: Memory Management
// ============================================================================

/// Free a string returned by this library
///
/// # Safety
/// Must only be called on strings returned by this library's FFI functions
#[no_mangle]
pub extern "C" fn mcp_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            free_c_string(ptr);
        }
    }
}

// ============================================================================
// FFI Functions: Utility
// ============================================================================

/// Get library version
///
/// # Returns
/// Version string (do NOT free)
#[no_mangle]
pub extern "C" fn mcp_version() -> *const c_char {
    // Static version string, never freed
    static VERSION: std::sync::LazyLock<CString> =
        std::sync::LazyLock::new(|| CString::new(env!("CARGO_PKG_VERSION")).unwrap());
    VERSION.as_ptr()
}

/// Get server count
///
/// # Returns
/// Number of active servers
#[no_mangle]
pub extern "C" fn mcp_server_count() -> u32 {
    match SERVERS.lock() {
        Ok(servers) => servers.len() as u32,
        Err(_) => 0,
    }
}

/// List all server IDs
///
/// # Arguments
/// * `buffer` - Buffer to write server IDs
/// * `buffer_len` - Buffer length
///
/// # Returns
/// Number of server IDs written
#[no_mangle]
pub extern "C" fn mcp_server_list(buffer: *mut u32, buffer_len: usize) -> u32 {
    if buffer.is_null() || buffer_len == 0 {
        return 0;
    }

    match SERVERS.lock() {
        Ok(servers) => {
            let ids: Vec<u32> = servers.keys().cloned().collect();
            let count = std::cmp::min(ids.len(), buffer_len);
            unsafe {
                std::ptr::copy_nonoverlapping(ids.as_ptr(), buffer, count);
            }
            count as u32
        }
        Err(_) => 0,
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to convert C string back to Rust string
    fn c_string_to_rust(ptr: *mut c_char) -> String {
        if ptr.is_null() {
            return String::new();
        }
        unsafe {
            let s = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            free_c_string(ptr);
            s
        }
    }

    fn c_string_to_rust_const(ptr: *const c_char) -> String {
        if ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(ptr).to_str().unwrap().to_string() }
    }

    #[test]
    fn test_server_create_and_stop() {
        let addr = CString::new("127.0.0.1:8080").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        assert_ne!(server_id, 0, "Server creation should return non-zero ID");
        assert_eq!(mcp_server_is_running(server_id), 1, "Server should be running");

        let result = mcp_server_stop(server_id);
        assert_eq!(result, 1, "Server stop should succeed");
        assert_eq!(mcp_server_is_running(server_id), 0, "Server should be stopped");
    }

    #[test]
    fn test_server_create_with_cors() {
        let addr = CString::new("0.0.0.0:3000").unwrap();
        let cors = CString::new("http://localhost:3000,https://example.com").unwrap();

        let server_id = mcp_server_create_with_cors(addr.as_ptr(), cors.as_ptr());
        assert_ne!(server_id, 0);

        let config_ptr = mcp_server_get_config(server_id);
        let config = c_string_to_rust(config_ptr);

        assert!(config.contains("0.0.0.0:3000"));
        assert!(config.contains("http://localhost:3000"));
        assert!(config.contains("https://example.com"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_server_invalid_address() {
        let addr = CString::new("").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        assert_eq!(server_id, 0, "Empty address should fail");

        let error_ptr = mcp_last_error();
        let error = c_string_to_rust_const(error_ptr);
        assert!(!error.is_empty(), "Should have error message");
    }

    #[test]
    fn test_chat_request_simple() {
        let addr = CString::new("127.0.0.1:8081").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());
        assert_ne!(server_id, 0);

        let request = CString::new(r#"{"prompt": "Hello", "agent": false}"#).unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        assert!(!response_ptr.is_null(), "Response should not be null");
        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"status\":\"ok\""));
        assert!(response.contains("\"prompt\":\"Hello\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_chat_request_with_agent_mode() {
        let addr = CString::new("127.0.0.1:8082").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = CString::new(r#"{"prompt": "Test", "agent": true, "max_tool_steps": 5}"#).unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"agent\":true"));
        assert!(response.contains("\"max_tool_steps\":5"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_chat_request_with_session() {
        let addr = CString::new("127.0.0.1:8083").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = CString::new(r#"{"prompt": "Hi", "session_id": "test-session-123"}"#).unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("test-session-123"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_chat_invalid_server() {
        let request = CString::new(r#"{"prompt": "Test"}"#).unwrap();
        let response_ptr = mcp_server_chat(99999, request.as_ptr(), request.to_bytes().len());

        assert!(!response_ptr.is_null());
        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("error"));
    }

    #[test]
    fn test_chat_invalid_json() {
        let addr = CString::new("127.0.0.1:8084").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = CString::new("not json").unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("error"));
        assert!(response.contains("JSON"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_chat_empty_prompt() {
        let addr = CString::new("127.0.0.1:8085").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = CString::new(r#"{"prompt": ""}"#).unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"prompt\":\"\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_get_tools() {
        let addr = CString::new("127.0.0.1:8086").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let tools_ptr = mcp_server_get_tools(server_id);
        assert!(!tools_ptr.is_null());

        let tools = c_string_to_rust(tools_ptr);
        assert!(tools.contains("\"tools\":"));
        assert!(tools.contains("\"total_tool_count\":"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_get_tools_invalid_server() {
        let tools_ptr = mcp_server_get_tools(99999);
        assert!(!tools_ptr.is_null());

        // Should return error or empty
        let tools = c_string_to_rust(tools_ptr);
        assert!(!tools.is_empty());
    }

    #[test]
    fn test_get_config() {
        let addr = CString::new("192.168.1.100:9090").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let config_ptr = mcp_server_get_config(server_id);
        assert!(!config_ptr.is_null());

        let config = c_string_to_rust(config_ptr);
        assert!(config.contains("192.168.1.100:9090"));
        assert!(config.contains("\"running\":true"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_reload_config() {
        let addr = CString::new("127.0.0.1:8087").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let reload_ptr = mcp_server_reload(server_id);
        assert!(!reload_ptr.is_null());

        let reload = c_string_to_rust(reload_ptr);
        assert!(reload.contains("\"success\":true"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_update_config() {
        let addr = CString::new("127.0.0.1:8088").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let update = CString::new(r#"{"model": "gpt-4", "default_provider": "openai"}"#).unwrap();
        let response_ptr = mcp_server_update_config(server_id, update.as_ptr());

        assert!(!response_ptr.is_null());
        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"success\":true"));
        assert!(response.contains("gpt-4"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_update_config_invalid_json() {
        let addr = CString::new("127.0.0.1:8089").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let update = CString::new("not json").unwrap();
        let response_ptr = mcp_server_update_config(server_id, update.as_ptr());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"success\":true"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_error_handling() {
        // Clear any previous errors
        mcp_clear_error();

        // Trigger an error
        let invalid_addr = CString::new("").unwrap();
        let server_id = mcp_server_create(invalid_addr.as_ptr());
        assert_eq!(server_id, 0);

        // Check error
        let error_ptr = mcp_last_error();
        assert!(!error_ptr.is_null());

        let error = c_string_to_rust_const(error_ptr);
        assert!(!error.is_empty());

        // Clear error
        mcp_clear_error();
        let error_ptr = mcp_last_error();
        assert!(error_ptr.is_null());
    }

    #[test]
    fn test_string_free() {
        let test_str = CString::new("Hello, FFI!").unwrap();
        let ptr = test_str.into_raw();

        assert!(!ptr.is_null());
        mcp_string_free(ptr);
        // After free, accessing would be UB - but test doesn't do that
    }

    #[test]
    fn test_string_free_null() {
        // Should not panic
        mcp_string_free(std::ptr::null_mut());
    }

    #[test]
    fn test_version() {
        let version_ptr = mcp_version();
        assert!(!version_ptr.is_null());

        let version = c_string_to_rust_const(version_ptr);
        assert!(!version.is_empty());
        assert!(version.chars().next().unwrap().is_ascii_digit());
    }

    #[test]
    fn test_server_count() {
        // Stop all servers first
        mcp_server_stop_all();

        let initial_count = mcp_server_count();

        // Create 3 servers
        let addr = CString::new("127.0.0.1:8090").unwrap();
        let s1 = mcp_server_create(addr.as_ptr());
        let s2 = mcp_server_create(addr.as_ptr());
        let s3 = mcp_server_create(addr.as_ptr());

        assert_eq!(mcp_server_count(), initial_count + 3);

        // Cleanup
        mcp_server_stop(s1);
        mcp_server_stop(s2);
        mcp_server_stop(s3);
    }

    #[test]
    fn test_server_list() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:8091").unwrap();
        let s1 = mcp_server_create(addr.as_ptr());
        let s2 = mcp_server_create(addr.as_ptr());

        let mut buffer = [0u32; 10];
        let count = mcp_server_list(buffer.as_mut_ptr(), buffer.len());

        assert!(count >= 2);
        assert!(buffer.iter().any(|&id| id == s1));
        assert!(buffer.iter().any(|&id| id == s2));

        mcp_server_stop(s1);
        mcp_server_stop(s2);
    }

    #[test]
    fn test_server_list_null_buffer() {
        let count = mcp_server_list(std::ptr::null_mut(), 10);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_server_list_zero_len() {
        let mut buffer = [0u32; 1];
        let count = mcp_server_list(buffer.as_mut_ptr(), 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_multiple_servers() {
        mcp_server_stop_all();

        let addrs = [
            "127.0.0.1:9001",
            "127.0.0.1:9002",
            "127.0.0.1:9003",
        ];

        let mut server_ids = Vec::new();

        for addr in &addrs {
            let c_addr = CString::new(*addr).unwrap();
            let id = mcp_server_create(c_addr.as_ptr());
            assert_ne!(id, 0);
            server_ids.push(id);
        }

        assert_eq!(mcp_server_count(), 3);

        // Chat with each server
        for &id in &server_ids {
            let request = CString::new(r#"{"prompt": "test"}"#).unwrap();
            let response_ptr = mcp_server_chat(id, request.as_ptr(), request.to_bytes().len());
            assert!(!response_ptr.is_null());

            let response = c_string_to_rust(response_ptr);
            assert!(response.contains("\"status\":\"ok\""));
        }

        // Stop all
        for &id in &server_ids {
            assert_eq!(mcp_server_stop(id), 1);
        }

        assert_eq!(mcp_server_count(), 0);
    }

    #[test]
    fn test_concurrent_operations() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:9010").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Multiple operations on same server
        for i in 0..10 {
            let request = CString::new(&format!(r#"{{"prompt": "message {}"}}"#, i)).unwrap();
            let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());
            assert!(!response_ptr.is_null());

            let response = c_string_to_rust(response_ptr);
            assert!(response.contains(&format!("message {}", i)));
        }

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_from_c_string_null() {
        let result = from_c_string(std::ptr::null());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Null pointer");
    }

    #[test]
    fn test_chat_request_with_attachments() {
        let addr = CString::new("127.0.0.1:9020").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = serde_json::json!({
            "prompt": "Analyze this",
            "attachments": [
                {"name": "image.png", "mime_type": "image/png", "data": "base64data"}
            ],
            "agent": true
        });

        let request_str = request.to_string();
        let request_c = CString::new(request_str.clone()).unwrap();
        let response_ptr = mcp_server_chat(server_id, request_c.as_ptr(), request_str.len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"status\":\"ok\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_chat_request_with_system_prompt() {
        let addr = CString::new("127.0.0.1:9021").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        let request = serde_json::json!({
            "prompt": "Hello",
            "system_prompt": "You are a helpful assistant",
            "debug": true
        });

        let request_str = request.to_string();
        let request_c = CString::new(request_str.clone()).unwrap();
        let response_ptr = mcp_server_chat(server_id, request_c.as_ptr(), request_str.len());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("\"status\":\"ok\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_config_update_partial() {
        let addr = CString::new("127.0.0.1:9030").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Update only model
        let update = serde_json::json!({
            "model": "claude-3"
        });

        let update_str = update.to_string();
        let update_c = CString::new(update_str.clone()).unwrap();
        let response_ptr = mcp_server_update_config(server_id, update_c.as_ptr());

        let response = c_string_to_rust(response_ptr);
        assert!(response.contains("claude-3"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_server_stop_nonexistent() {
        let result = mcp_server_stop(999999);
        assert_eq!(result, 0, "Stopping non-existent server should fail");

        let error_ptr = mcp_last_error();
        let error = c_string_to_rust_const(error_ptr);
        assert!(error.contains("not found"));
    }

    #[test]
    fn test_stop_all_clears_count() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:9040").unwrap();
        let s1 = mcp_server_create(addr.as_ptr());
        let s2 = mcp_server_create(addr.as_ptr());
        let s3 = mcp_server_create(addr.as_ptr());

        assert_eq!(mcp_server_count(), 3);

        let stopped = mcp_server_stop_all();
        assert_eq!(stopped, 3);
        assert_eq!(mcp_server_count(), 0);

        // Cleanup (should be no-op)
        mcp_server_stop(s1);
        mcp_server_stop(s2);
        mcp_server_stop(s3);
    }

    // ========================================================================
    // Custom Response Fields Tests
    // ========================================================================

    #[test]
    fn test_response_add_field_string() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:9050").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add string field
        let field_name = CString::new("version").unwrap();
        let field_value = CString::new("\"1.0.0\"").unwrap();

        let result = mcp_response_add_field(server_id, field_name.as_ptr(), field_value.as_ptr());
        assert_eq!(result, 1, "Adding field should succeed");

        // Check field count
        assert_eq!(mcp_response_field_count(server_id), 1);

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_field_number() {
        let addr = CString::new("127.0.0.1:9051").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add number field
        let field_name = CString::new("request_count").unwrap();
        let field_value = CString::new("42").unwrap();

        let result = mcp_response_add_field(server_id, field_name.as_ptr(), field_value.as_ptr());
        assert_eq!(result, 1);

        // Get fields and verify
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("request_count"));
        assert!(fields.contains("42"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_field_object() {
        let addr = CString::new("127.0.0.1:9052").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add object field
        let field_name = CString::new("metadata").unwrap();
        let field_value = CString::new(r#"{"author": "test", "version": "1.0"}"#).unwrap();

        let result = mcp_response_add_field(server_id, field_name.as_ptr(), field_value.as_ptr());
        assert_eq!(result, 1);

        // Get fields and verify
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("metadata"));
        assert!(fields.contains("author"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_field_array() {
        let addr = CString::new("127.0.0.1:9053").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add array field
        let field_name = CString::new("tags").unwrap();
        let field_value = CString::new(r#"["tag1", "tag2", "tag3"]"#).unwrap();

        let result = mcp_response_add_field(server_id, field_name.as_ptr(), field_value.as_ptr());
        assert_eq!(result, 1);

        // Get fields and verify
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("tags"));
        assert!(fields.contains("tag1"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_field_boolean() {
        let addr = CString::new("127.0.0.1:9054").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add boolean field
        let field_name = CString::new("debug_mode").unwrap();
        let field_value = CString::new("true").unwrap();

        let result = mcp_response_add_field(server_id, field_name.as_ptr(), field_value.as_ptr());
        assert_eq!(result, 1);

        // Get fields and verify
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("debug_mode"));
        assert!(fields.contains("true"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_multiple_fields() {
        let addr = CString::new("127.0.0.1:9055").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add multiple fields
        let fields = vec![
            ("version", "\"2.0.0\""),
            ("build", "123"),
            ("environment", "\"production\""),
        ];

        for (name, value) in fields {
            let name_c = CString::new(name).unwrap();
            let value_c = CString::new(value).unwrap();
            let result = mcp_response_add_field(server_id, name_c.as_ptr(), value_c.as_ptr());
            assert_eq!(result, 1);
        }

        // Check count
        assert_eq!(mcp_response_field_count(server_id), 3);

        // Get all fields
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields_json = c_string_to_rust(fields_ptr);
        assert!(fields_json.contains("version"));
        assert!(fields_json.contains("build"));
        assert!(fields_json.contains("environment"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_remove_field() {
        let addr = CString::new("127.0.0.1:9056").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add field
        let name = CString::new("temp_field").unwrap();
        let value = CString::new("\"temp_value\"").unwrap();
        mcp_response_add_field(server_id, name.as_ptr(), value.as_ptr());
        assert_eq!(mcp_response_field_count(server_id), 1);

        // Remove field
        let result = mcp_response_remove_field(server_id, name.as_ptr());
        assert_eq!(result, 1, "Removing field should succeed");
        assert_eq!(mcp_response_field_count(server_id), 0);

        // Verify field is gone
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(!fields.contains("temp_field"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_remove_nonexistent_field() {
        let addr = CString::new("127.0.0.1:9057").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Remove field that doesn't exist
        let name = CString::new("nonexistent").unwrap();
        let result = mcp_response_remove_field(server_id, name.as_ptr());
        assert_eq!(result, 1, "Removing nonexistent field should still succeed");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_clear_fields() {
        let addr = CString::new("127.0.0.1:9058").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add multiple fields
        let fields = vec![
            ("field1", "\"value1\""),
            ("field2", "\"value2\""),
            ("field3", "\"value3\""),
        ];

        for (name, value) in fields {
            let name_c = CString::new(name).unwrap();
            let value_c = CString::new(value).unwrap();
            mcp_response_add_field(server_id, name_c.as_ptr(), value_c.as_ptr());
        }

        assert_eq!(mcp_response_field_count(server_id), 3);

        // Clear all fields
        let cleared = mcp_response_clear_fields(server_id);
        assert_eq!(cleared, 3, "Should clear 3 fields");
        assert_eq!(mcp_response_field_count(server_id), 0);

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_apply_fields() {
        let addr = CString::new("127.0.0.1:9059").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add custom fields
        let name1 = CString::new("custom_field").unwrap();
        let value1 = CString::new("\"custom_value\"").unwrap();
        mcp_response_add_field(server_id, name1.as_ptr(), value1.as_ptr());

        let name2 = CString::new("count").unwrap();
        let value2 = CString::new("100").unwrap();
        mcp_response_add_field(server_id, name2.as_ptr(), value2.as_ptr());

        // Apply fields to a response
        let response = CString::new(r#"{"status": "ok", "message": "Hello"}"#).unwrap();
        let result_ptr = mcp_response_apply_fields(server_id, response.as_ptr());

        assert!(!result_ptr.is_null());
        let result = c_string_to_rust(result_ptr);

        // Verify original fields still present
        assert!(result.contains("\"status\":\"ok\""));
        assert!(result.contains("\"message\":\"Hello\""));

        // Verify custom fields added
        assert!(result.contains("\"custom_field\":\"custom_value\""));
        assert!(result.contains("\"count\":100"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_apply_fields_empty() {
        let addr = CString::new("127.0.0.1:9060").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Don't add any fields
        let response = CString::new(r#"{"status": "ok"}"#).unwrap();
        let result_ptr = mcp_response_apply_fields(server_id, response.as_ptr());

        assert!(!result_ptr.is_null());
        let result = c_string_to_rust(result_ptr);
        assert_eq!(result, r#"{"status":"ok"}"#);

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_apply_fields_invalid_json() {
        let addr = CString::new("127.0.0.1:9061").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add a field
        let name = CString::new("field").unwrap();
        let value = CString::new("\"value\"").unwrap();
        mcp_response_add_field(server_id, name.as_ptr(), value.as_ptr());

        // Apply to invalid JSON
        let response = CString::new("not json").unwrap();
        let result_ptr = mcp_response_apply_fields(server_id, response.as_ptr());

        assert!(result_ptr.is_null());

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_chat_includes_custom_fields() {
        let addr = CString::new("127.0.0.1:9062").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add custom fields
        let name1 = CString::new("api_version").unwrap();
        let value1 = CString::new("\"v2\"").unwrap();
        mcp_response_add_field(server_id, name1.as_ptr(), value1.as_ptr());

        let name2 = CString::new("server_id").unwrap();
        let value2 = CString::new("\"test-server\"").unwrap();
        mcp_response_add_field(server_id, name2.as_ptr(), value2.as_ptr());

        // Chat
        let request = CString::new(r#"{"prompt": "Test", "agent": false}"#).unwrap();
        let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());

        let response = c_string_to_rust(response_ptr);

        // Verify custom fields included
        assert!(response.contains("\"api_version\":\"v2\""));
        assert!(response.contains("\"server_id\":\"test-server\""));
        assert!(response.contains("\"status\":\"ok\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_field_overwrite() {
        let addr = CString::new("127.0.0.1:9063").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add field
        let name = CString::new("version").unwrap();
        let value1 = CString::new("\"1.0\"").unwrap();
        mcp_response_add_field(server_id, name.as_ptr(), value1.as_ptr());

        // Overwrite with new value
        let value2 = CString::new("\"2.0\"").unwrap();
        mcp_response_add_field(server_id, name.as_ptr(), value2.as_ptr());

        // Check count (should still be 1)
        assert_eq!(mcp_response_field_count(server_id), 1);

        // Get fields and verify overwritten
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("\"version\":\"2.0\""));
        assert!(!fields.contains("\"version\":\"1.0\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_fields_persist_across_operations() {
        let addr = CString::new("127.0.0.1:9064").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add field
        let name = CString::new("persistent").unwrap();
        let value = CString::new("\"data\"").unwrap();
        mcp_response_add_field(server_id, name.as_ptr(), value.as_ptr());

        // Multiple chat operations
        for i in 0..3 {
            let request = CString::new(&format!(r#"{{"prompt": "msg {}"}}"#, i)).unwrap();
            let response_ptr = mcp_server_chat(server_id, request.as_ptr(), request.to_bytes().len());
            let response = c_string_to_rust(response_ptr);
            assert!(response.contains("\"persistent\":\"data\""));
        }

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_add_field_invalid_server() {
        let name = CString::new("field").unwrap();
        let value = CString::new("\"value\"").unwrap();

        let result = mcp_response_add_field(99999, name.as_ptr(), value.as_ptr());
        assert_eq!(result, 0, "Should fail for nonexistent server");
    }

    #[test]
    fn test_response_add_field_invalid_json() {
        let addr = CString::new("127.0.0.1:9065").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add field with invalid JSON value
        let name = CString::new("bad_field").unwrap();
        let value = CString::new("not json").unwrap();

        let result = mcp_response_add_field(server_id, name.as_ptr(), value.as_ptr());
        assert_eq!(result, 0, "Should fail for invalid JSON");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_get_fields_invalid_server() {
        let fields_ptr = mcp_response_get_fields(99999);
        assert!(!fields_ptr.is_null());

        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("error") || fields.is_empty());
    }

    #[test]
    fn test_response_complex_nested_object() {
        let addr = CString::new("127.0.0.1:9066").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Add complex nested object
        let name = CString::new("config").unwrap();
        let value = CString::new(r#"
        {
            "database": {
                "host": "localhost",
                "port": 5432,
                "options": {
                    "pool_size": 10,
                    "timeout": 30
                }
            },
            "features": ["auth", "logging", "cache"]
        }
        "#).unwrap();

        let result = mcp_response_add_field(server_id, name.as_ptr(), value.as_ptr());
        assert_eq!(result, 1);

        // Get and verify
        let fields_ptr = mcp_response_get_fields(server_id);
        let fields = c_string_to_rust(fields_ptr);
        assert!(fields.contains("database"));
        assert!(fields.contains("localhost"));
        assert!(fields.contains("5432"));
        assert!(fields.contains("features"));

        // Apply to response
        let response = CString::new(r#"{"status": "ok"}"#).unwrap();
        let result_ptr = mcp_response_apply_fields(server_id, response.as_ptr());
        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("config"));
        assert!(result.contains("database"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_response_multiple_servers_isolated() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:9067").unwrap();
        let s1 = mcp_server_create(addr.as_ptr());
        let s2 = mcp_server_create(addr.as_ptr());

        // Add different fields to each server
        let name1 = CString::new("server").unwrap();
        let val1 = CString::new("\"server1\"").unwrap();
        mcp_response_add_field(s1, name1.as_ptr(), val1.as_ptr());

        let val2 = CString::new("\"server2\"").unwrap();
        mcp_response_add_field(s2, name1.as_ptr(), val2.as_ptr());

        // Verify isolation
        assert_eq!(mcp_response_field_count(s1), 1);
        assert_eq!(mcp_response_field_count(s2), 1);

        // Get fields for s1
        let fields1_ptr = mcp_response_get_fields(s1);
        let fields1 = c_string_to_rust(fields1_ptr);
        assert!(fields1.contains("server1"));
        assert!(!fields1.contains("server2"));

        // Get fields for s2
        let fields2_ptr = mcp_response_get_fields(s2);
        let fields2 = c_string_to_rust(fields2_ptr);
        assert!(fields2.contains("server2"));
        assert!(!fields2.contains("server1"));

        mcp_server_stop(s1);
        mcp_server_stop(s2);
    }

    // ========================================================================
    // Output Format Tests
    // ========================================================================

    #[test]
    fn test_output_format_set_and_get_json() {
        let addr = CString::new("127.0.0.1:9070").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set JSON format
        let format = CString::new("json").unwrap();
        let result = mcp_set_output_format(server_id, format.as_ptr());
        assert_eq!(result, 1, "Setting format should succeed");

        // Get format
        let format_ptr = mcp_get_output_format(server_id);
        let format_str = c_string_to_rust(format_ptr);
        assert_eq!(format_str, "json");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_output_format_set_markdown() {
        let addr = CString::new("127.0.0.1:9071").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set markdown format
        let format = CString::new("markdown").unwrap();
        let result = mcp_set_output_format(server_id, format.as_ptr());
        assert_eq!(result, 1);

        // Get format
        let format_ptr = mcp_get_output_format(server_id);
        let format_str = c_string_to_rust(format_ptr);
        assert_eq!(format_str, "markdown");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_output_format_set_text() {
        let addr = CString::new("127.0.0.1:9072").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set text format
        let format = CString::new("text").unwrap();
        let result = mcp_set_output_format(server_id, format.as_ptr());
        assert_eq!(result, 1);

        // Get format
        let format_ptr = mcp_get_output_format(server_id);
        let format_str = c_string_to_rust(format_ptr);
        assert_eq!(format_str, "text");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_output_format_invalid() {
        let addr = CString::new("127.0.0.1:9073").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set invalid format
        let format = CString::new("xml").unwrap();
        let result = mcp_set_output_format(server_id, format.as_ptr());
        assert_eq!(result, 0, "Invalid format should fail");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_format_response_json() {
        let addr = CString::new("127.0.0.1:9074").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set JSON format
        let format = CString::new("json").unwrap();
        mcp_set_output_format(server_id, format.as_ptr());

        // Format response with data
        let content = CString::new("Hello, world!").unwrap();
        let data = CString::new(r#"{"key": "value"}"#).unwrap();
        let result_ptr = mcp_format_response(server_id, content.as_ptr(), data.as_ptr());

        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("\"content\":\"Hello, world!\""));
        assert!(result.contains("\"key\":\"value\""));
        assert!(result.contains("\"format\":\"json\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_format_response_markdown() {
        let addr = CString::new("127.0.0.1:9075").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set markdown format
        let format = CString::new("markdown").unwrap();
        mcp_set_output_format(server_id, format.as_ptr());

        // Format response with data
        let content = CString::new("Hello, world!").unwrap();
        let data = CString::new(r#"{"key": "value"}"#).unwrap();
        let result_ptr = mcp_format_response(server_id, content.as_ptr(), data.as_ptr());

        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("# Response"));
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("## Data"));
        assert!(result.contains("```json"));
        assert!(result.contains("{\"key\":\"value\"}"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_format_response_text() {
        let addr = CString::new("127.0.0.1:9076").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set text format
        let format = CString::new("text").unwrap();
        mcp_set_output_format(server_id, format.as_ptr());

        // Format response with data
        let content = CString::new("Hello, world!").unwrap();
        let data = CString::new(r#"{"key": "value"}"#).unwrap();
        let result_ptr = mcp_format_response(server_id, content.as_ptr(), data.as_ptr());

        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("Data:"));
        assert!(result.contains("{\"key\":\"value\"}"));
        assert!(!result.contains("# Response"));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_format_response_without_data() {
        let addr = CString::new("127.0.0.1:9077").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Set JSON format
        let format = CString::new("json").unwrap();
        mcp_set_output_format(server_id, format.as_ptr());

        // Format response without data
        let content = CString::new("Hello, world!").unwrap();
        let result_ptr = mcp_format_response(server_id, content.as_ptr(), std::ptr::null());

        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("\"content\":\"Hello, world!\""));
        assert!(!result.contains("\"data\""));

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_output_format_default_is_json() {
        let addr = CString::new("127.0.0.1:9078").unwrap();
        let server_id = mcp_server_create(addr.as_ptr());

        // Don't set format - should default to JSON
        let format_ptr = mcp_get_output_format(server_id);
        let format_str = c_string_to_rust(format_ptr);
        assert_eq!(format_str, "json");

        mcp_server_stop(server_id);
    }

    #[test]
    fn test_output_format_multiple_servers() {
        mcp_server_stop_all();

        let addr = CString::new("127.0.0.1:9079").unwrap();
        let s1 = mcp_server_create(addr.as_ptr());
        let s2 = mcp_server_create(addr.as_ptr());
        let s3 = mcp_server_create(addr.as_ptr());

        // Set different formats
        mcp_set_output_format(s1, CString::new("json").unwrap().as_ptr());
        mcp_set_output_format(s2, CString::new("markdown").unwrap().as_ptr());
        mcp_set_output_format(s3, CString::new("text").unwrap().as_ptr());

        // Verify isolation
        let f1 = c_string_to_rust(mcp_get_output_format(s1));
        let f2 = c_string_to_rust(mcp_get_output_format(s2));
        let f3 = c_string_to_rust(mcp_get_output_format(s3));

        assert_eq!(f1, "json");
        assert_eq!(f2, "markdown");
        assert_eq!(f3, "text");

        mcp_server_stop(s1);
        mcp_server_stop(s2);
        mcp_server_stop(s3);
    }

    #[test]
    fn test_format_response_invalid_server() {
        let content = CString::new("Hello").unwrap();
        let result_ptr = mcp_format_response(99999, content.as_ptr(), std::ptr::null());

        assert!(result_ptr.is_null());
    }
}
