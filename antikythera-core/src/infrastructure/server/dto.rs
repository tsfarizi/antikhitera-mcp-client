use crate::application::agent::{AgentStep, ServerGuidance, ToolContext, ToolDescriptor};
use crate::config::{ModelProviderConfig, ToolConfig};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Attachment for multimodal input (image or file)
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct Attachment {
    /// File name
    pub name: String,
    /// MIME type (e.g., "image/png", "application/pdf")
    pub mime_type: String,
    /// Base64 encoded file data
    pub data: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RestChatRequest {
    /// The user's message/prompt
    pub prompt: String,
    /// Optional file/image attachments (base64 encoded)
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    /// Optional system prompt override
    pub system_prompt: Option<String>,
    /// Session ID for conversation continuity
    pub session_id: Option<String>,
    /// Enable agent mode with tool execution
    #[serde(default)]
    pub agent: bool,
    /// Maximum tool execution steps in agent mode
    #[serde(default)]
    pub max_tool_steps: Option<usize>,
    /// Debug mode: if true, returns verbose response (logs, steps). Defaults to false.
    #[serde(default)]
    pub debug: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestChatResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<String>>,
    pub session_id: String,
    #[schema(value_type = Object)]
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_steps: Option<Vec<AgentStep>>,
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

#[derive(Debug, Clone, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
    pub config: Option<ConfigResponse>,
}
