use crate::agent::{AgentStep, ServerGuidance, ToolContext, ToolDescriptor};
use crate::config::ToolConfig;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RestChatRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub agent: bool,
    #[serde(default)]
    pub max_tool_steps: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestChatResponse {
    pub session_id: String,
    pub content: String,
    pub tool_steps: Vec<AgentStep>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ToolInventoryResponse {
    pub tools: Vec<ToolDescriptor>,
    pub servers: Vec<ServerGuidance>,
}

impl From<ToolContext> for ToolInventoryResponse {
    fn from(context: ToolContext) -> Self {
        Self {
            tools: context.tools,
            servers: context.servers,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigResponse {
    pub model: String,
    pub system_prompt: Option<String>,
    pub prompt_template: String,
    pub tools: Vec<ToolConfig>,
    pub raw: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigUpdateRequest {
    pub model: String,
    pub system_prompt: Option<String>,
    pub prompt_template: String,
}
