use crate::domain::types::MessagePart;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AgentStep {
    pub tool: String,
    pub input: Value,
    pub success: bool,
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

#[derive(Debug, Clone, Deserialize)]
pub struct AgentOptions {
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    pub attachments: Vec<MessagePart>,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            system_prompt: None,
            session_id: None,
            max_steps: default_max_steps(),
            attachments: Vec::new(),
        }
    }
}

fn default_max_steps() -> usize {
    DEFAULT_MAX_STEPS
}
