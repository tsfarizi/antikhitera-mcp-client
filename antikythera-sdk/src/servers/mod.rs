//! Server Management Feature Slice
//!
//! This module provides types, registry, validation, and FFI bindings
//! for managing MCP server configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

use crate::sdk_logging::get_sdk_logger;
use antikythera_log::LogLevel;

fn server_log(level: LogLevel, message: &str) {
    get_sdk_logger("mcp_servers").log_with_source(level, "mcp_servers", message);
}

// ============================================================================
// Types
// ============================================================================

/// MCP Server transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerTransport {
    Stdio,
    Http,
    Sse,
}

/// MCP Server configuration with strict validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique server identifier
    pub name: String,
    /// Transport mechanism
    pub transport: ServerTransport,
    /// Command to execute (for stdio) or URL (for http/sse)
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Connection timeout in milliseconds
    pub timeout_ms: Option<u32>,
    /// Whether server is enabled
    pub enabled: bool,
    /// Optional server description
    pub description: Option<String>,
}

/// Server validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerValidationResult {
    /// Whether configuration is valid
    pub valid: bool,
    /// List of validation errors (empty if valid)
    pub errors: Vec<String>,
    /// Server name that was validated
    pub server_name: String,
}

/// Server operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerOperationResult {
    /// Whether operation succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Affected server name
    pub server_name: String,
    /// Number of tools affected
    pub tools_affected: u32,
}

// ============================================================================
// Validation
// ============================================================================

impl McpServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> ServerValidationResult {
        let mut errors = Vec::new();

        // Name validation
        if self.name.is_empty() {
            errors.push("Server name cannot be empty".to_string());
        }
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(
                "Server name can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            );
        }

        // Command validation
        if self.command.is_empty() {
            errors.push("Command cannot be empty".to_string());
        }

        // HTTP/SSE URL validation
        if matches!(self.transport, ServerTransport::Http | ServerTransport::Sse)
            && !self.command.starts_with("http://")
            && !self.command.starts_with("https://")
        {
            errors.push(
                "HTTP/SSE servers require a valid URL starting with http:// or https://"
                    .to_string(),
            );
        }

        // Timeout validation
        if let Some(timeout) = self.timeout_ms
            && timeout == 0
        {
            errors.push("Timeout must be greater than 0".to_string());
        }

        ServerValidationResult {
            valid: errors.is_empty(),
            errors,
            server_name: self.name.clone(),
        }
    }
}

// ============================================================================
// Registry
// ============================================================================

/// Global server registry
static SERVERS: LazyLock<Mutex<HashMap<String, McpServerConfig>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// FFI Bindings
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

/// Add a new MCP server configuration
pub fn mcp_add_server(config_json: *const c_char) -> *mut c_char {
    server_log(LogLevel::Debug, "mcp_add_server called");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            return to_c_string(&format!(
                r#"{{"valid":false,"errors":["{}"],"server_name":""}}"#,
                e
            ));
        }
    };

    let config: McpServerConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            let result = ServerValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                server_name: String::new(),
            };
            return serialize_result(&result);
        }
    };

    let validation = config.validate();
    if !validation.valid {
        server_log(LogLevel::Info, &format!("Server validation failed for '{}': {:?}", config.name, validation.errors));
        return serialize_result(&validation);
    }

    let name = config.name.clone();
    match SERVERS.lock() {
        Ok(mut servers) => {
            if servers.contains_key(&name) {
                let result = ServerValidationResult {
                    valid: false,
                    errors: vec![format!("Server '{}' already exists", name)],
                    server_name: name,
                };
                return serialize_result(&result);
            }

            servers.insert(name.clone(), config);
            server_log(LogLevel::Info, &format!("Server added: '{}'", name));
            serialize_result(&ServerValidationResult {
                valid: true,
                errors: vec![],
                server_name: name,
            })
        }
        Err(e) => {
            server_log(LogLevel::Error, &format!("Lock error in mcp_add_server: {e}"));
            serialize_result(&ServerValidationResult {
                valid: false,
                errors: vec![format!("Failed to lock registry: {}", e)],
                server_name: String::new(),
            })
        }
    }
}

/// Remove an MCP server by name
pub fn mcp_remove_server(name: *const c_char) -> *mut c_char {
    server_log(LogLevel::Debug, "mcp_remove_server called");
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => {
            return serialize_result(&ServerOperationResult {
                success: false,
                error_message: Some(e),
                server_name: String::new(),
                tools_affected: 0,
            });
        }
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            if servers.remove(&name_str).is_some() {
                server_log(LogLevel::Info, &format!("Server removed: '{}'", name_str));
                serialize_result(&ServerOperationResult {
                    success: true,
                    error_message: None,
                    server_name: name_str,
                    tools_affected: 0,
                })
            } else {
                serialize_result(&ServerOperationResult {
                    success: false,
                    error_message: Some(format!("Server '{}' not found", name_str)),
                    server_name: name_str,
                    tools_affected: 0,
                })
            }
        }
        Err(e) => {
            server_log(LogLevel::Error, &format!("Lock error in mcp_remove_server: {e}"));
            serialize_result(&ServerOperationResult {
                success: false,
                error_message: Some(format!("Failed to lock registry: {}", e)),
                server_name: name_str,
                tools_affected: 0,
            })
        },
    }
}

/// List all configured MCP servers
pub fn mcp_list_servers() -> *mut c_char {
    match SERVERS.lock() {
        Ok(servers) => {
            let configs: Vec<&McpServerConfig> = servers.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get configuration for a specific server
pub fn mcp_get_server(name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(config) = servers.get(&name_str) {
                serialize_result(config)
            } else {
                to_c_string(&format!(
                    r#"{{"error": "Server '{}' not found"}}"#,
                    name_str
                ))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Validate server configuration without adding
pub fn mcp_validate_server(config_json: *const c_char) -> *mut c_char {
    server_log(LogLevel::Debug, "mcp_validate_server called");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            return serialize_result(&ServerValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                server_name: String::new(),
            });
        }
    };

    let config: McpServerConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            return serialize_result(&ServerValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                server_name: String::new(),
            });
        }
    };

    let result = config.validate();
    if result.valid {
        server_log(LogLevel::Info, &format!("Server validated: '{}'", config.name));
    } else {
        server_log(LogLevel::Info, &format!("Server validation failed for '{}': {:?}", config.name, result.errors));
    }
    serialize_result(&result)
}

/// Export all servers configuration as JSON
pub fn mcp_export_servers_config() -> *mut c_char {
    match SERVERS.lock() {
        Ok(servers) => {
            let configs: Vec<&McpServerConfig> = servers.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Import servers configuration from JSON
pub fn mcp_import_servers_config(config_json: *const c_char) -> *mut c_char {
    server_log(LogLevel::Debug, "mcp_import_servers_config called");
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            return serialize_result(&ServerOperationResult {
                success: false,
                error_message: Some(e),
                server_name: "import".to_string(),
                tools_affected: 0,
            });
        }
    };

    let configs: Vec<McpServerConfig> = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            return serialize_result(&ServerOperationResult {
                success: false,
                error_message: Some(format!("Invalid JSON: {}", e)),
                server_name: "import".to_string(),
                tools_affected: 0,
            });
        }
    };

    let count = configs.len();
    match SERVERS.lock() {
        Ok(mut servers) => {
            for config in configs {
                let name = config.name.clone();
                servers.insert(name, config);
            }
            server_log(LogLevel::Info, &format!("Imported {count} server configurations"));
            serialize_result(&ServerOperationResult {
                success: true,
                error_message: None,
                server_name: "import".to_string(),
                tools_affected: count as u32,
            })
        }
        Err(e) => {
            server_log(LogLevel::Error, &format!("Lock error in mcp_import_servers_config: {e}"));
            serialize_result(&ServerOperationResult {
                success: false,
                error_message: Some(format!("Failed to lock registry: {}", e)),
                server_name: "import".to_string(),
                tools_affected: 0,
            })
        },
    }
}
