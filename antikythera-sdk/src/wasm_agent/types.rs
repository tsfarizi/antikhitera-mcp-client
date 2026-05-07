//! WASM Agent Types
//!
//! Types for WASM agent that processes LLM responses.
//! WASM does NOT call LLM APIs - host does that.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[path = "prompt_variables.rs"]
pub mod prompt_variables;
#[path = "stream_types.rs"]
pub mod stream_types;
#[path = "tool_registry.rs"]
pub mod tool_registry;

pub use prompt_variables::*;
pub use stream_types::*;
pub use tool_registry::*;

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
    Final { response: serde_json::Value },
    /// Retry with error
    Retry { error: String },
}

// ============================================================================
// Advanced Context Management
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    #[default]
    KeepNewest,
    KeepBalanced,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPolicy {
    pub max_history_messages: usize,
    pub summarize_after_messages: usize,
    pub summary_max_chars: usize,
    #[serde(default)]
    pub truncation_strategy: TruncationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSummary {
    pub version: u64,
    pub text: String,
    pub source_messages: usize,
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
    /// Rolling summary for long context
    #[serde(default)]
    pub rolling_summary: Option<ContextSummary>,
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
            rolling_summary: None,
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
    /// Default context policy
    #[serde(default)]
    pub context_policy: ContextPolicy,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            verbose: false,
            auto_execute_tools: true,
            session_timeout_secs: 300,
            session_id: format!("session-{}", chrono::Utc::now().timestamp_millis()),
            context_policy: ContextPolicy {
                max_history_messages: 24,
                summarize_after_messages: 12,
                summary_max_chars: 1200,
                truncation_strategy: TruncationStrategy::KeepNewest,
            },
        }
    }
}
