use crate::agent::{AgentStep, ServerGuidance, ToolContext, ToolDescriptor};
use crate::config::{ModelProviderConfig, ToolConfig};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RestChatRequest {
    pub prompt: String,
    pub provider: Option<String>,
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
    pub logs: Vec<String>,
    pub session_id: String,
    pub content: String,
    pub provider: String,
    pub model: String,
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
    pub default_provider: String,
    pub system_prompt: Option<String>,
    pub prompt_template: String,
    pub tools: Vec<ToolConfig>,
    pub providers: Vec<ModelProviderConfig>,
    pub raw: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigUpdateRequest {
    pub model: String,
    pub default_provider: String,
    pub system_prompt: Option<String>,
    pub prompt_template: String,
}
