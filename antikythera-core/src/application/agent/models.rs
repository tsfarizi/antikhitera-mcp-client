use crate::domain::types::MessagePart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

const DEFAULT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
pub struct AgentStep {
    pub tool: String,
    #[schema(value_type = Object)]
    pub input: Value,
    pub success: bool,
    #[schema(value_type = Object)]
    pub output: Value,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub logs: Vec<String>,
    pub session_id: String,
    pub response: Value,
    pub steps: Vec<AgentStep>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentOptions {
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    pub attachments: Vec<MessagePart>,
}

fn default_max_steps() -> usize {
    DEFAULT_MAX_STEPS
}
