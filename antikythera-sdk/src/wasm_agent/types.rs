//! WASM Agent Types
//!
//! Types for WASM agent that processes LLM responses.
//! WASM does NOT call LLM APIs - host does that.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Agent Actions
// ============================================================================

/// Action the agent wants to take
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    /// Call a tool
    CallTool {
        tool: String,
        input: serde_json::Value,
    },
    /// Final response to user
    Final {
        response: serde_json::Value,
    },
    /// Retry with error
    Retry {
        error: String,
    },
}

// ============================================================================
// Agent State
// ============================================================================

/// Agent session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Session ID
    pub session_id: String,
    /// Current step number
    pub current_step: u32,
    /// Message history (user + assistant + tool results)
    pub message_history: Vec<AgentMessage>,
    /// Tool call results
    pub tool_results: HashMap<String, serde_json::Value>,
    /// Agent configuration
    pub config: AgentConfig,
}

impl AgentState {
    /// Create new session
    pub fn new(config: AgentConfig) -> Self {
        Self {
            session_id: config.session_id.clone(),
            current_step: 0,
            message_history: Vec::new(),
            tool_results: HashMap::new(),
            config,
        }
    }

    /// Add message to history
    pub fn add_message(&mut self, message: AgentMessage) {
        self.message_history.push(message);
    }

    /// Record tool result
    pub fn record_tool_result(&mut self, tool_name: String, result: serde_json::Value) {
        self.tool_results.insert(tool_name, result);
        self.current_step += 1;
    }

    /// Check if max steps exceeded
    pub fn is_max_steps_exceeded(&self) -> bool {
        self.current_step >= self.config.max_steps
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}

// ============================================================================
// Messages
// ============================================================================

/// Message in conversation (for WASM agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional tool call info
    pub tool_call: Option<ToolCall>,
    /// Optional tool result
    pub tool_result: Option<ToolResult>,
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Step ID
    pub step_id: u32,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name
    pub name: String,
    /// Success status
    pub success: bool,
    /// Output
    pub output: serde_json::Value,
    /// Error message
    pub error: Option<String>,
    /// Step ID
    pub step_id: u32,
}

// ============================================================================
// Agent Configuration
// ============================================================================

/// Agent behavior config (matches WIT agent-config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum steps
    pub max_steps: u32,
    /// Verbose logging
    pub verbose: bool,
    /// Auto-execute tools
    pub auto_execute_tools: bool,
    /// Session timeout (seconds)
    pub session_timeout_secs: u32,
    /// Session ID
    pub session_id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            verbose: false,
            auto_execute_tools: true,
            session_timeout_secs: 300,
            session_id: format!("session-{}", chrono::Utc::now().timestamp_millis()),
        }
    }
}

// ============================================================================
// Prompt Types
// ============================================================================

/// Prompt template variables
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptVariables {
    pub custom_instruction: Option<String>,
    pub language_guidance: Option<String>,
    pub tool_guidance: Option<String>,
    pub json_schema: Option<String>,
}

impl PromptVariables {
    /// Render template with variables
    pub fn render(&self, template: &str) -> String {
        let mut result = template.to_string();

        if let Some(val) = &self.custom_instruction {
            result = result.replace("{{custom_instruction}}", val);
        } else {
            result = result.replace("{{custom_instruction}}\n\n", "");
        }

        if let Some(val) = &self.language_guidance {
            result = result.replace("{{language_guidance}}", val);
        } else {
            result = result.replace("\n\n{{language_guidance}}", "");
        }

        if let Some(val) = &self.tool_guidance {
            result = result.replace("{{tool_guidance}}", val);
        } else {
            result = result.replace("\n\n{{tool_guidance}}", "");
        }

        if let Some(val) = &self.json_schema {
            result.push_str("\n\n");
            result.push_str(val);
        }

        result
    }
}
