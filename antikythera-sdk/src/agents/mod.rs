//! Agent Management Feature Slice
//!
//! This module provides types, registry, validation, and FFI bindings
//! for managing multi-agent configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, LazyLock};

// ============================================================================
// Types
// ============================================================================

/// Agent type/role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    GeneralAssistant,
    CodeReviewer,
    DataAnalyst,
    Researcher,
    Custom,
}

/// Agent skill level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Expert,
}

/// Agent capability descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Capability name
    pub name: String,
    /// Skill level for this capability
    pub level: SkillLevel,
    /// Description of capability
    pub description: String,
}

/// Agent configuration with strict validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub id: String,
    /// Agent type/role
    #[serde(rename = "agent-type")]
    pub agent_type: AgentType,
    /// Display name
    pub name: String,
    /// Agent description
    pub description: Option<String>,
    /// Model provider to use
    pub model_provider: String,
    /// Model name to use
    pub model: String,
    /// Maximum steps allowed
    pub max_steps: u32,
    /// Whether agent can call tools
    pub can_call_tools: bool,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Custom system prompt (overrides default)
    pub custom_prompt: Option<String>,
    /// Temperature for LLM
    pub temperature: Option<f32>,
    /// Whether agent is enabled
    pub enabled: bool,
}

/// Agent validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentValidationResult {
    /// Whether configuration is valid
    pub valid: bool,
    /// List of validation errors
    pub errors: Vec<String>,
    /// Agent ID that was validated
    pub agent_id: String,
}

/// Agent status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID
    pub id: String,
    /// Agent name
    pub name: String,
    /// Whether agent is currently active
    pub active: bool,
    /// Current session ID (if active)
    pub session_id: Option<String>,
    /// Number of tasks completed
    pub tasks_completed: u32,
    /// Number of tasks failed
    pub tasks_failed: u32,
}

/// Agent task request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskRequest {
    /// Task description/prompt
    pub task: String,
    /// Optional session ID for continuity
    pub session_id: Option<String>,
    /// Maximum steps for this task
    pub max_steps: Option<u32>,
    /// Whether to allow tool calls
    pub allow_tools: Option<bool>,
}

/// Agent task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskResult {
    /// Task output
    pub response: String,
    /// Whether task succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Number of steps executed
    pub steps_executed: u32,
    /// Tools called during task
    pub tools_called: Vec<String>,
    /// Session ID (if any)
    pub session_id: Option<String>,
}

/// Multi-agent orchestration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    /// Final synthesized response
    pub response: String,
    /// Whether orchestration succeeded
    pub success: bool,
    /// Agent contributions (agent_id -> contribution)
    pub contributions: Vec<(String, String)>,
    /// Total steps across all agents
    pub total_steps: u32,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

// ============================================================================
// Validation
// ============================================================================

impl AgentConfig {
    /// Validate agent configuration
    pub fn validate(&self) -> AgentValidationResult {
        let mut errors = Vec::new();

        // ID validation
        if self.id.is_empty() {
            errors.push("Agent ID cannot be empty".to_string());
        }
        if !self.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push("Agent ID can only contain alphanumeric characters, hyphens, and underscores".to_string());
        }

        // Name validation
        if self.name.is_empty() {
            errors.push("Agent name cannot be empty".to_string());
        }

        // Model validation
        if self.model_provider.is_empty() {
            errors.push("Model provider cannot be empty".to_string());
        }
        if self.model.is_empty() {
            errors.push("Model name cannot be empty".to_string());
        }

        // Max steps validation
        if self.max_steps == 0 {
            errors.push("Max steps must be greater than 0".to_string());
        }

        // Temperature validation
        if let Some(temp) = self.temperature {
            if temp < 0.0 || temp > 2.0 {
                errors.push("Temperature must be between 0.0 and 2.0".to_string());
            }
        }

        AgentValidationResult {
            valid: errors.is_empty(),
            errors,
            agent_id: self.id.clone(),
        }
    }
}

// ============================================================================
// Registry
// ============================================================================

/// Agent registry
static AGENTS: LazyLock<Mutex<HashMap<String, AgentConfig>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Agent status tracking
static AGENT_STATUS: LazyLock<Mutex<HashMap<String, AgentStatus>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get mutable access to agent registry (for tests)
#[allow(dead_code)]
pub fn agents_lock() -> std::sync::MutexGuard<'static, HashMap<String, AgentConfig>> {
    AGENTS.lock().unwrap()
}

/// Get mutable access to agent status registry (for tests)
#[allow(dead_code)]
pub fn agent_status_lock() -> std::sync::MutexGuard<'static, HashMap<String, AgentStatus>> {
    AGENT_STATUS.lock().unwrap()
}

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

/// Register a new agent configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_register_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let validation = config.validate();
    if !validation.valid {
        return serialize_result(&validation);
    }

    let id = config.id.clone();
    match AGENTS.lock() {
        Ok(mut agents) => {
            if agents.contains_key(&id) {
                return serialize_result(&AgentValidationResult {
                    valid: false,
                    errors: vec![format!("Agent '{}' already exists", id)],
                    agent_id: id,
                });
            }

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

            serialize_result(&AgentValidationResult {
                valid: true,
                errors: vec![],
                agent_id: id,
            })
        }
        Err(e) => serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Failed to lock registry: {}", e)],
            agent_id: String::new(),
        }),
    }
}

/// Unregister an agent by ID
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
pub extern "C" fn mcp_validate_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    serialize_result(&config.validate())
}

/// Export all agents configuration as JSON
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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
