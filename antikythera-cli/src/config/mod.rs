//! CLI Configuration
//!
//! Configuration for CLI/native testing only (Gemini & Ollama only).
//! NOT used by WASM - WASM receives LLM responses from host.

use serde::{Deserialize, Serialize};
use std::path::Path;

// ============================================================================
// CLI Configuration (for testing only)
// ============================================================================

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
}

/// Provider configuration (CLI testing only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProviderConfig {
    /// Provider ID
    pub id: String,
    /// Provider type (gemini or ollama only)
    #[serde(rename = "type")]
    pub provider_type: String,
    /// API endpoint
    pub endpoint: String,
    /// API key (or env var name) - None for Ollama
    pub api_key: String,
    /// Available models
    pub models: Vec<ModelInfo>,
}

/// Server configuration (REST server settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address
    pub bind: String,
    /// CORS origins
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".to_string(),
            cors_origins: Vec::new(),
        }
    }
}

/// Complete CLI configuration (for testing only)
/// Only GEMINI and OLLAMA providers supported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Providers (gemini or ollama)
    pub providers: Vec<CliProviderConfig>,
    /// Default provider
    pub default_provider: String,
    /// Default model
    pub model: String,
    /// Server settings
    pub server: ServerConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            default_provider: "ollama".to_string(),
            model: "llama3".to_string(),
            server: ServerConfig::default(),
        }
    }
}

// ============================================================================
// Postcard Serialization
// ============================================================================

/// CLI config file path
pub const CLI_CONFIG_PATH: &str = "cli-config.pc";

/// Serialize CLI config
pub fn config_to_postcard(config: &CliConfig) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(config).map_err(|e| format!("Serialize error: {}", e))
}

/// Deserialize CLI config
pub fn config_from_postcard(data: &[u8]) -> Result<CliConfig, String> {
    postcard::from_bytes(data).map_err(|e| format!("Deserialize error: {}", e))
}

/// Load CLI config
pub fn load_config(path: Option<&Path>) -> Result<CliConfig, String> {
    let config_path = path.unwrap_or(Path::new(CLI_CONFIG_PATH));
    if !config_path.exists() {
        return Err(format!("Config not found: {}", config_path.display()));
    }
    let data = std::fs::read(config_path)
        .map_err(|e| format!("Read error: {}", e))?;
    config_from_postcard(&data)
}

/// Save CLI config
pub fn save_config(config: &CliConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = path.unwrap_or(Path::new(CLI_CONFIG_PATH));
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create dir error: {}", e))?;
    }
    let data = config_to_postcard(config)?;
    std::fs::write(config_path, data)
        .map_err(|e| format!("Write error: {}", e))
}

/// Check if CLI config exists
pub fn config_exists() -> bool {
    Path::new(CLI_CONFIG_PATH).exists()
}

/// Initialize default CLI config
pub fn init_default_config() -> Result<CliConfig, String> {
    let config = CliConfig::default();
    save_config(&config, None)?;
    Ok(config)
}
