//! Basic agent and streaming types

use serde::{Deserialize, Serialize};

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

/// SDK-level streaming mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamingModeOption {
    Token,
    Event,
    #[default]
    Mixed,
}

/// Host-facing streaming options for incremental output.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamingOptions {
    #[serde(default)]
    pub mode: StreamingModeOption,
    #[serde(default = "default_true")]
    pub include_final_response: bool,
    #[serde(default)]
    pub max_buffered_events: Option<usize>,
}

pub(crate) const fn default_true() -> bool {
    true
}

impl StreamingOptions {
    pub fn validate(&self) -> Result<(), String> {
        let errors = super::validation::validate_streaming_options_collect(self);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}
