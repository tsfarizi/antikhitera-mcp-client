use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

const DEFAULT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone, Serialize, ToSchema)]
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
    pub session_id: String,
    pub response: String,
    pub steps: Vec<AgentStep>,
}

#[derive(Debug, Clone)]
pub struct AgentOptions {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    pub max_steps: usize,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            model: None,
            system_prompt: None,
            session_id: None,
            max_steps: DEFAULT_MAX_STEPS,
        }
    }
}
