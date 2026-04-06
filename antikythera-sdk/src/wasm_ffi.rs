//! FFI Implementation for Server and Agent Management
//!
//! Provides C-compatible functions for host languages to manage
//! MCP servers and multi-agent configurations.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

use crate::wasm_types::*;

// ============================================================================
// Global State
// ============================================================================

/// Server registry
static SERVERS: Mutex<HashMap<String, McpServerConfig>> = Mutex::new(HashMap::new());

/// Agent registry
static AGENTS: Mutex<HashMap<String, AgentConfig>> = Mutex::new(HashMap::new());

/// Agent status tracking
static AGENT_STATUS: Mutex<HashMap<String, AgentStatus>> = Mutex::new(HashMap::new());

// ============================================================================
// Helper Functions
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

// ============================================================================
// Server Management FFI Functions
// ============================================================================

/// Add a new MCP server configuration
/// Validates configuration before adding
#[no_mangle]
pub extern "C" fn mcp_add_server(
    config_json: *const c_char,
) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"valid":false,"errors":["{}"],"server_name":""}}"#, e)),
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

    // Validate configuration
    let validation = config.validate();
    if !validation.valid {
        return serialize_result(&validation);
    }

    // Add to registry
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
            let result = ServerValidationResult {
                valid: true,
                errors: vec![],
                server_name: name,
            };
            serialize_result(&result)
        }
        Err(e) => {
            let result = ServerValidationResult {
                valid: false,
                errors: vec![format!("Failed to lock registry: {}", e)],
                server_name: String::new(),
            };
            serialize_result(&result)
        }
    }
}

/// Remove an MCP server by name
#[no_mangle]
pub extern "C" fn mcp_remove_server(name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => {
            let result = ServerOperationResult {
                success: false,
                error_message: Some(e),
                server_name: String::new(),
                tools_affected: 0,
            };
            return serialize_result(&result);
        }
    };

    match SERVERS.lock() {
        Ok(mut servers) => {
            if servers.remove(&name_str).is_some() {
                let result = ServerOperationResult {
                    success: true,
                    error_message: None,
                    server_name: name_str,
                    tools_affected: 0,
                };
                serialize_result(&result)
            } else {
                let result = ServerOperationResult {
                    success: false,
                    error_message: Some(format!("Server '{}' not found", name_str)),
                    server_name: name_str,
                    tools_affected: 0,
                };
                serialize_result(&result)
            }
        }
        Err(e) => {
            let result = ServerOperationResult {
                success: false,
                error_message: Some(format!("Failed to lock registry: {}", e)),
                server_name: name_str,
                tools_affected: 0,
            };
            serialize_result(&result)
        }
    }
}

/// List all configured MCP servers
#[no_mangle]
pub extern "C" fn mcp_list_servers() -> *mut c_char {
    match SERVERS.lock() {
        Ok(servers) => {
            let configs: Vec<&McpServerConfig> = servers.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get configuration for a specific server
#[no_mangle]
pub extern "C" fn mcp_get_server(name: *const c_char) -> *mut c_char {
    let name_str = match from_c_string(name) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match SERVERS.lock() {
        Ok(servers) => {
            if let Some(config) = servers.get(&name_str) {
                serialize_result(config)
            } else {
                to_c_string(&format!(r#"{{"error": "Server '{}' not found"}}"#, name_str))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Validate server configuration without adding
#[no_mangle]
pub extern "C" fn mcp_validate_server(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            let result = ServerValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                server_name: String::new(),
            };
            return serialize_result(&result);
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

    serialize_result(&config.validate())
}

/// Export all servers configuration as JSON
#[no_mangle]
pub extern "C" fn mcp_export_servers_config() -> *mut c_char {
    match SERVERS.lock() {
        Ok(servers) => {
            let configs: Vec<&McpServerConfig> = servers.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Import servers configuration from JSON
#[no_mangle]
pub extern "C" fn mcp_import_servers_config(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            let result = ServerOperationResult {
                success: false,
                error_message: Some(e),
                server_name: "import".to_string(),
                tools_affected: 0,
            };
            return serialize_result(&result);
        }
    };

    let configs: Vec<McpServerConfig> = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            let result = ServerOperationResult {
                success: false,
                error_message: Some(format!("Invalid JSON: {}", e)),
                server_name: "import".to_string(),
                tools_affected: 0,
            };
            return serialize_result(&result);
        }
    };

    let count = configs.len();
    match SERVERS.lock() {
        Ok(mut servers) => {
            for config in configs {
                let name = config.name.clone();
                servers.insert(name, config);
            }
            let result = ServerOperationResult {
                success: true,
                error_message: None,
                server_name: "import".to_string(),
                tools_affected: count as u32,
            };
            serialize_result(&result)
        }
        Err(e) => {
            let result = ServerOperationResult {
                success: false,
                error_message: Some(format!("Failed to lock registry: {}", e)),
                server_name: "import".to_string(),
                tools_affected: 0,
            };
            serialize_result(&result)
        }
    }
}

// ============================================================================
// Agent Management FFI Functions
// ============================================================================

/// Register a new agent configuration
#[no_mangle]
pub extern "C" fn mcp_register_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            let result = AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            };
            return serialize_result(&result);
        }
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            let result = AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            };
            return serialize_result(&result);
        }
    };

    // Validate configuration
    let validation = config.validate();
    if !validation.valid {
        return serialize_result(&validation);
    }

    // Add to registry
    let id = config.id.clone();
    match AGENTS.lock() {
        Ok(mut agents) => {
            if agents.contains_key(&id) {
                let result = AgentValidationResult {
                    valid: false,
                    errors: vec![format!("Agent '{}' already exists", id)],
                    agent_id: id,
                };
                return serialize_result(&result);
            }

            // Initialize status
            let status = AgentStatus {
                id: id.clone(),
                name: config.name.clone(),
                active: false,
                session_id: None,
                tasks_completed: 0,
                tasks_failed: 0,
            };

            AGENT_STATUS.lock().unwrap().insert(id.clone(), status);
            agents.insert(id.clone(), config);

            let result = AgentValidationResult {
                valid: true,
                errors: vec![],
                agent_id: id,
            };
            serialize_result(&result)
        }
        Err(e) => {
            let result = AgentValidationResult {
                valid: false,
                errors: vec![format!("Failed to lock registry: {}", e)],
                agent_id: String::new(),
            };
            serialize_result(&result)
        }
    }
}

/// Unregister an agent by ID
#[no_mangle]
pub extern "C" fn mcp_unregister_agent(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match AGENTS.lock() {
        Ok(mut agents) => {
            if agents.remove(&id_str).is_some() {
                AGENT_STATUS.lock().unwrap().remove(&id_str);
                to_c_string(r#"{"success": true}"#)
            } else {
                to_c_string(&format!(r#"{{"error": "Agent '{}' not found"}}"#, id_str))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// List all registered agents
#[no_mangle]
pub extern "C" fn mcp_list_agents() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get configuration for a specific agent
#[no_mangle]
pub extern "C" fn mcp_get_agent(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match AGENTS.lock() {
        Ok(agents) => {
            if let Some(config) = agents.get(&id_str) {
                serialize_result(config)
            } else {
                to_c_string(&format!(r#"{{"error": "Agent '{}' not found"}}"#, id_str))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get runtime status of all agents
#[no_mangle]
pub extern "C" fn mcp_get_agent_status() -> *mut c_char {
    match AGENT_STATUS.lock() {
        Ok(statuses) => {
            let status_list: Vec<&AgentStatus> = statuses.values().collect();
            serialize_result(&status_list)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Validate agent configuration without registering
#[no_mangle]
pub extern "C" fn mcp_validate_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            let result = AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            };
            return serialize_result(&result);
        }
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            let result = AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            };
            return serialize_result(&result);
        }
    };

    serialize_result(&config.validate())
}

/// Export all agents configuration as JSON
#[no_mangle]
pub extern "C" fn mcp_export_agents_config() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Import agents configuration from JSON
#[no_mangle]
pub extern "C" fn mcp_import_agents_config(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    let configs: Vec<AgentConfig> = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return to_c_string(&format!(r#"{{"error": "Invalid JSON: {}"}}"#, e)),
    };

    let count = configs.len();
    match AGENTS.lock() {
        Ok(mut agents) => {
            for config in configs {
                let id = config.id.clone();
                let status = AgentStatus {
                    id: id.clone(),
                    name: config.name.clone(),
                    active: false,
                    session_id: None,
                    tasks_completed: 0,
                    tasks_failed: 0,
                };

                AGENT_STATUS.lock().unwrap().insert(id.clone(), status);
                agents.insert(id, config);
            }
            to_c_string(&format!(r#"{{"success": true, "imported": {}}}"#, count))
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Helper to convert C string back to Rust string
    fn c_string_to_rust(ptr: *mut c_char) -> String {
        if ptr.is_null() {
            return String::new();
        }
        unsafe {
            let s = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            drop(CString::from_raw(ptr));
            s
        }
    }

    // ========================================================================
    // Server Management Tests
    // ========================================================================

    #[test]
    fn test_add_valid_server() {
        // Clear servers first
        SERVERS.lock().unwrap().clear();

        let config = McpServerConfig {
            name: "test-server".to_string(),
            transport: ServerTransport::Stdio,
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: vec![("NODE_ENV".to_string(), "production".to_string())],
            timeout_ms: Some(5000),
            enabled: true,
            description: Some("Test MCP Server".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let c_json = CString::new(json).unwrap();

        let result_ptr = mcp_add_server(c_json.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

        assert!(validation.valid);
        assert_eq!(validation.server_name, "test-server");
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_add_duplicate_server() {
        SERVERS.lock().unwrap().clear();

        let config = McpServerConfig {
            name: "duplicate-server".to_string(),
            transport: ServerTransport::Stdio,
            command: "node".to_string(),
            args: vec![],
            env: vec![],
            timeout_ms: None,
            enabled: true,
            description: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let c_json = CString::new(json.clone()).unwrap();

        // Add first time
        let result_ptr = mcp_add_server(c_json.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();
        assert!(validation.valid);

        // Add second time (should fail)
        let c_json2 = CString::new(json).unwrap();
        let result_ptr = mcp_add_server(c_json2.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|e| e.contains("already exists")));
    }

    #[test]
    fn test_remove_server() {
        SERVERS.lock().unwrap().clear();

        let config = McpServerConfig {
            name: "to-remove".to_string(),
            transport: ServerTransport::Http,
            command: "http://localhost:3000".to_string(),
            args: vec![],
            env: vec![],
            timeout_ms: Some(3000),
            enabled: true,
            description: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let c_json = CString::new(json).unwrap();
        mcp_add_server(c_json.as_ptr());

        // Remove server
        let name = CString::new("to-remove").unwrap();
        let result_ptr = mcp_remove_server(name.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let op_result: ServerOperationResult = serde_json::from_str(&result).unwrap();

        assert!(op_result.success);
        assert_eq!(op_result.server_name, "to-remove");
    }

    #[test]
    fn test_list_servers() {
        SERVERS.lock().unwrap().clear();

        let config1 = McpServerConfig {
            name: "server-1".to_string(),
            transport: ServerTransport::Stdio,
            command: "cmd1".to_string(),
            args: vec![],
            env: vec![],
            timeout_ms: None,
            enabled: true,
            description: None,
        };

        let config2 = McpServerConfig {
            name: "server-2".to_string(),
            transport: ServerTransport::Sse,
            command: "http://localhost:4000".to_string(),
            args: vec![],
            env: vec![],
            timeout_ms: Some(5000),
            enabled: true,
            description: None,
        };

        let json1 = serde_json::to_string(&config1).unwrap();
        let json2 = serde_json::to_string(&config2).unwrap();

        mcp_add_server(CString::new(json1).unwrap().as_ptr());
        mcp_add_server(CString::new(json2).unwrap().as_ptr());

        let result_ptr = mcp_list_servers();
        let result = c_string_to_rust(result_ptr);
        let servers: Vec<McpServerConfig> = serde_json::from_str(&result).unwrap();

        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn test_validate_invalid_server() {
        // Empty name should fail
        let config = serde_json::json!({
            "name": "",
            "transport": "Stdio",
            "command": "",
            "args": [],
            "env": [],
            "timeout_ms": null,
            "enabled": true,
            "description": null
        });

        let json = config.to_string();
        let c_json = CString::new(json).unwrap();

        let result_ptr = mcp_validate_server(c_json.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_export_import_servers() {
        SERVERS.lock().unwrap().clear();

        let config = McpServerConfig {
            name: "export-test".to_string(),
            transport: ServerTransport::Stdio,
            command: "test".to_string(),
            args: vec!["arg1".to_string()],
            env: vec![],
            timeout_ms: None,
            enabled: true,
            description: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        mcp_add_server(CString::new(json).unwrap().as_ptr());

        // Export
        let export_ptr = mcp_export_servers_config();
        let export_json = c_string_to_rust(export_ptr);

        // Clear servers
        SERVERS.lock().unwrap().clear();

        // Import
        let import_ptr = mcp_import_servers_config(CString::new(export_json).unwrap().as_ptr());
        let import_result = c_string_to_rust(import_ptr);
        let op_result: ServerOperationResult = serde_json::from_str(&import_result).unwrap();

        assert!(op_result.success);
        assert_eq!(op_result.tools_affected, 1);
    }

    // ========================================================================
    // Agent Management Tests
    // ========================================================================

    #[test]
    fn test_register_valid_agent() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "test-agent".to_string(),
            agent_type: AgentType::GeneralAssistant,
            name: "Test Agent".to_string(),
            description: Some("A test agent".to_string()),
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![
                AgentCapability {
                    name: "coding".to_string(),
                    level: SkillLevel::Expert,
                    description: "Expert coding assistant".to_string(),
                }
            ],
            custom_prompt: None,
            temperature: Some(0.7),
            enabled: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let c_json = CString::new(json).unwrap();

        let result_ptr = mcp_register_agent(c_json.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: AgentValidationResult = serde_json::from_str(&result).unwrap();

        assert!(validation.valid);
        assert_eq!(validation.agent_id, "test-agent");
    }

    #[test]
    fn test_register_duplicate_agent() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "duplicate-agent".to_string(),
            agent_type: AgentType::CodeReviewer,
            name: "Duplicate Agent".to_string(),
            description: None,
            model_provider: "anthropic".to_string(),
            model: "claude-3".to_string(),
            max_steps: 5,
            can_call_tools: false,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let c_json = CString::new(json.clone()).unwrap();

        // Register first time
        mcp_register_agent(c_json.as_ptr());

        // Register second time (should fail)
        let c_json2 = CString::new(json).unwrap();
        let result_ptr = mcp_register_agent(c_json2.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: AgentValidationResult = serde_json::from_str(&result).unwrap();

        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|e| e.contains("already exists")));
    }

    #[test]
    fn test_list_agents() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config1 = AgentConfig {
            id: "agent-1".to_string(),
            agent_type: AgentType::GeneralAssistant,
            name: "Agent 1".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        let config2 = AgentConfig {
            id: "agent-2".to_string(),
            agent_type: AgentType::DataAnalyst,
            name: "Agent 2".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 15,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        mcp_register_agent(CString::new(serde_json::to_string(&config1).unwrap()).unwrap().as_ptr());
        mcp_register_agent(CString::new(serde_json::to_string(&config2).unwrap()).unwrap().as_ptr());

        let result_ptr = mcp_list_agents();
        let result = c_string_to_rust(result_ptr);
        let agents: Vec<AgentConfig> = serde_json::from_str(&result).unwrap();

        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_get_agent() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "get-test".to_string(),
            agent_type: AgentType::Researcher,
            name: "Get Test Agent".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        mcp_register_agent(CString::new(serde_json::to_string(&config).unwrap()).unwrap().as_ptr());

        let id = CString::new("get-test").unwrap();
        let result_ptr = mcp_get_agent(id.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let agent: AgentConfig = serde_json::from_str(&result).unwrap();

        assert_eq!(agent.id, "get-test");
        assert_eq!(agent.agent_type, AgentType::Researcher);
    }

    #[test]
    fn test_validate_invalid_agent() {
        // Empty ID should fail
        let config = serde_json::json!({
            "id": "",
            "agent-type": "GeneralAssistant",
            "name": "Test",
            "description": null,
            "model-provider": "openai",
            "model": "gpt-4",
            "max-steps": 0,
            "can-call-tools": true,
            "capabilities": [],
            "custom-prompt": null,
            "temperature": null,
            "enabled": true
        });

        let json = config.to_string();
        let c_json = CString::new(json).unwrap();

        let result_ptr = mcp_validate_agent(c_json.as_ptr());
        let result = c_string_to_rust(result_ptr);
        let validation: AgentValidationResult = serde_json::from_str(&result).unwrap();

        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_unregister_agent() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "unregister-test".to_string(),
            agent_type: AgentType::GeneralAssistant,
            name: "Unregister Test".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        mcp_register_agent(CString::new(serde_json::to_string(&config).unwrap()).unwrap().as_ptr());

        let id = CString::new("unregister-test").unwrap();
        let result_ptr = mcp_unregister_agent(id.as_ptr());
        let result = c_string_to_rust(result_ptr);

        assert!(result.contains("\"success\": true"));

        // Verify agent is gone
        let result_ptr = mcp_get_agent(id.as_ptr());
        let result = c_string_to_rust(result_ptr);
        assert!(result.contains("error"));
    }

    #[test]
    fn test_get_agent_status() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "status-test".to_string(),
            agent_type: AgentType::GeneralAssistant,
            name: "Status Test".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        mcp_register_agent(CString::new(serde_json::to_string(&config).unwrap()).unwrap().as_ptr());

        let result_ptr = mcp_get_agent_status();
        let result = c_string_to_rust(result_ptr);
        let statuses: Vec<AgentStatus> = serde_json::from_str(&result).unwrap();

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].id, "status-test");
        assert!(!statuses[0].active);
        assert_eq!(statuses[0].tasks_completed, 0);
    }

    #[test]
    fn test_export_import_agents() {
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        let config = AgentConfig {
            id: "export-test-agent".to_string(),
            agent_type: AgentType::CodeReviewer,
            name: "Export Test Agent".to_string(),
            description: None,
            model_provider: "anthropic".to_string(),
            model: "claude-3".to_string(),
            max_steps: 5,
            can_call_tools: false,
            capabilities: vec![],
            custom_prompt: None,
            temperature: None,
            enabled: true,
        };

        mcp_register_agent(CString::new(serde_json::to_string(&config).unwrap()).unwrap().as_ptr());

        // Export
        let export_ptr = mcp_export_agents_config();
        let export_json = c_string_to_rust(export_ptr);

        // Clear agents
        AGENTS.lock().unwrap().clear();
        AGENT_STATUS.lock().unwrap().clear();

        // Import
        let import_ptr = mcp_import_agents_config(CString::new(export_json).unwrap().as_ptr());
        let import_result = c_string_to_rust(import_ptr);
        let import_result_json: serde_json::Value = serde_json::from_str(&import_result).unwrap();

        assert_eq!(import_result_json["success"], true);
        assert_eq!(import_result_json["imported"], 1);
    }

    #[test]
    fn test_server_validation_edge_cases() {
        // Test invalid transport
        SERVERS.lock().unwrap().clear();

        let config = McpServerConfig {
            name: "valid-name".to_string(),
            transport: ServerTransport::Stdio,
            command: "valid-cmd".to_string(),
            args: vec![],
            env: vec![],
            timeout_ms: Some(0), // Invalid timeout
            enabled: true,
            description: None,
        };

        let validation = config.validate();
        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|e| e.contains("Timeout")));
    }

    #[test]
    fn test_agent_validation_edge_cases() {
        // Test invalid temperature
        let config = AgentConfig {
            id: "valid-id".to_string(),
            agent_type: AgentType::Custom,
            name: "Valid Name".to_string(),
            description: None,
            model_provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            can_call_tools: true,
            capabilities: vec![],
            custom_prompt: None,
            temperature: Some(3.0), // Invalid temperature (> 2.0)
            enabled: true,
        };

        let validation = config.validate();
        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|e| e.contains("Temperature")));
    }
}
