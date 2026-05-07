use serde::{Deserialize, Serialize};

use crate::wasm_agent::types::{ContextPolicy, ContextSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RunnerConfigInput {
    pub max_steps: Option<u32>,
    pub verbose: Option<bool>,
    pub auto_execute_tools: Option<bool>,
    pub session_timeout_secs: Option<u32>,
    pub max_in_memory_sessions: Option<usize>,
    pub session_id: Option<String>,
    pub context_policy: Option<ContextPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PrepareUserTurnInput {
    pub prompt: String,
    pub session_id: Option<String>,
    pub system_prompt: Option<String>,
    pub force_json: Option<bool>,
    pub metadata_json: Option<String>,
    pub correlation_id: Option<String>,
    pub context_policy: Option<ContextPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PreparedTurn {
    pub session_id: String,
    pub step: u32,
    pub prompt: String,
    pub system_prompt: String,
    pub force_json: bool,
    pub metadata_json: Option<String>,
    pub correlation_id: Option<String>,
    pub summary_handoff: Option<ContextSummary>,
    pub messages_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CommitResult {
    pub session_id: String,
    pub step: u32,
    pub action: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ToolResultInput {
    pub tool_name: String,
    pub success: bool,
    pub output_json: String,
    pub error_message: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ContextPolicyUpdateInput {
    pub policy: ContextPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ArchivedSessionRecord {
    pub archived_at_ms: i64,
    pub reason: String,
}
