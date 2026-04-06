//! WASM Component Types for Server and Agent Management
//!
//! These types mirror the WIT definitions exactly to ensure type safety
//! and enable automatic serialization/deserialization.

use serde::{Deserialize, Serialize};

// ============================================================================
// MCP Server Management Types
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

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server name
    pub name: String,
    /// Whether server is currently running
    pub running: bool,
    /// Number of tools provided by this server
    pub tool_count: u32,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Uptime in seconds (if running)
    pub uptime_secs: Option<u32>,
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
// Multi-Agent Management Types
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
// Validation Helpers
// ============================================================================

impl McpServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> ServerValidationResult {
        let mut errors = Vec::new();

        // Name validation
        if self.name.is_empty() {
            errors.push("Server name cannot be empty".to_string());
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push("Server name can only contain alphanumeric characters, hyphens, and underscores".to_string());
        }

        // Command validation
        if self.command.is_empty() {
            errors.push("Command cannot be empty".to_string());
        }

        // HTTP/SSE URL validation
        if matches!(self.transport, ServerTransport::Http | ServerTransport::Sse) {
            if !self.command.starts_with("http://") && !self.command.starts_with("https://") {
                errors.push("HTTP/SSE servers require a valid URL starting with http:// or https://".to_string());
            }
        }

        // Timeout validation
        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                errors.push("Timeout must be greater than 0".to_string());
            }
        }

        ServerValidationResult {
            valid: errors.is_empty(),
            errors,
            server_name: self.name.clone(),
        }
    }
}

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
