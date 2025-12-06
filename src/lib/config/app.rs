use super::error::ConfigError;
use super::provider::ModelProviderConfig;
use super::server::ServerConfig;
use super::tool::ToolConfig;
use std::path::Path;

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
