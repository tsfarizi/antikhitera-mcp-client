use super::error::ConfigError;
use super::provider::ModelProviderConfig;
use super::server::ServerConfig;
use super::tool::ToolConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// REST server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestServerConfig {
    /// Server bind address (e.g., "127.0.0.1:8080")
    #[serde(default = "default_bind")]
    pub bind: String,
    /// CORS allowed origins
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// API documentation servers
    #[serde(default)]
    pub docs: Vec<DocServerConfig>,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

impl Default for RestServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            cors_origins: Vec::new(),
            docs: Vec::new(),
        }
    }
}

/// API documentation server entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocServerConfig {
    pub url: String,
    pub description: String,
}

/// Configurable prompts for agent behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// Guidance when tools are available
    pub tool_guidance: Option<String>,
    /// Guidance when no tools match the request
    pub fallback_guidance: Option<String>,
    /// Message sent to LLM when JSON parsing fails (retry prompt)
    pub json_retry_message: Option<String>,
    /// Instruction for tool result formatting
    pub tool_result_instruction: Option<String>,
}

impl PromptsConfig {
    /// Default tool guidance (English)
    pub fn default_tool_guidance() -> &'static str {
        "You have access to the following tools. Use them only when necessary to fulfill the user request:"
    }

    /// Default fallback guidance (English)
    pub fn default_fallback_guidance() -> &'static str {
        "If the request is outside the scope of available tools, apologize politely and explain your limitations."
    }

    /// Default JSON retry message (English)
    pub fn default_json_retry_message() -> &'static str {
        "System Error: Invalid JSON format returned. Please output ONLY the raw JSON object for the tool call or final response. Do not use Markdown blocks or explanations."
    }

    /// Default tool result instruction (English)
    pub fn default_tool_result_instruction() -> &'static str {
        "Provide a valid JSON response: use {\"action\":\"call_tool\",\"tool\":\"...\",\"input\":{...}} for tool calls or {\"action\":\"final\",\"response\":\"...\"} for final answers. Do not include any text outside the JSON structure."
    }

    /// Get tool guidance with fallback to default
    pub fn tool_guidance(&self) -> &str {
        self.tool_guidance
            .as_deref()
            .unwrap_or(Self::default_tool_guidance())
    }

    /// Get fallback guidance with fallback to default
    pub fn fallback_guidance(&self) -> &str {
        self.fallback_guidance
            .as_deref()
            .unwrap_or(Self::default_fallback_guidance())
    }

    /// Get JSON retry message with fallback to default
    pub fn json_retry_message(&self) -> &str {
        self.json_retry_message
            .as_deref()
            .unwrap_or(Self::default_json_retry_message())
    }

    /// Get tool result instruction with fallback to default
    pub fn tool_result_instruction(&self) -> &str {
        self.tool_result_instruction
            .as_deref()
            .unwrap_or(Self::default_tool_result_instruction())
    }
}

/// Application configuration loaded from client.toml
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub default_provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub prompt_template: String,
    pub providers: Vec<ModelProviderConfig>,
    /// REST server settings (CORS, docs)
    pub rest_server: RestServerConfig,
    /// Configurable prompts for agent behavior
    pub prompts: PromptsConfig,
}

impl AppConfig {
    /// Load configuration from a file path (or default path if None)
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        super::loader::load_config(path)
    }

    /// Get the prompt template
    pub fn prompt_template(&self) -> &str {
        &self.prompt_template
    }

    /// Convert configuration to TOML string
    pub fn to_raw_toml(&self) -> String {
        super::serializer::to_raw_toml_string(self)
    }
}
