//! Unified Postcard-based Configuration
//!
//! All configuration is stored as a single Postcard binary file.
//! CLI and FFI provide full access to all config fields.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Unified Configuration Structure
// ============================================================================

/// Complete application configuration (single Postcard blob)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// REST server settings
    pub server: ServerConfig,
    /// LLM providers
    pub providers: Vec<ProviderConfig>,
    /// Default provider and model
    pub model: ModelConfig,
    /// All prompt templates
    pub prompts: PromptsConfig,
    /// Agent behavior settings
    pub agent: AgentConfig,
    /// Custom key-value pairs for extensibility
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            providers: Vec::new(),
            model: ModelConfig::default(),
            prompts: PromptsConfig::default(),
            agent: AgentConfig::default(),
            custom: HashMap::new(),
        }
    }
}

// ============================================================================
// Server Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address (e.g., "127.0.0.1:8080")
    pub bind: String,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// API documentation servers
    pub docs: Vec<DocServerConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".to_string(),
            cors_origins: Vec::new(),
            docs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocServerConfig {
    pub url: String,
    pub description: String,
}

// ============================================================================
// Provider Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Unique provider ID
    pub id: String,
    /// Provider type (openai, anthropic, ollama, gemini, etc.)
    pub provider_type: String,
    /// API endpoint URL
    pub endpoint: String,
    /// API key reference (env var name)
    pub api_key: String,
    /// Available models
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
}

// ============================================================================
// Model Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default provider ID
    pub default_provider: String,
    /// Default model name
    pub model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_provider: "ollama".to_string(),
            model: "llama3".to_string(),
        }
    }
}

// ============================================================================
// Prompts Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    pub template: String,
    pub tool_guidance: String,
    pub fallback_guidance: String,
    pub json_retry_message: String,
    pub tool_result_instruction: String,
    pub agent_instructions: String,
    pub ui_instructions: String,
    pub language_instructions: String,
    pub agent_max_steps_error: String,
    pub no_tools_guidance: String,
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            template: "You are a helpful AI assistant.\n\n{{custom_instruction}}\n\n{{language_guidance}}\n\n{{tool_guidance}}".to_string(),
            tool_guidance: "You have access to the following tools. Use them only when necessary to fulfill the user request:".to_string(),
            fallback_guidance: "If the request is outside the scope of available tools, apologize politely and explain your limitations.".to_string(),
            json_retry_message: "System Error: Invalid JSON format returned. Please output ONLY the raw JSON object for the tool call or final response. Do not use Markdown blocks or explanations.".to_string(),
            tool_result_instruction: "Tool execution complete. Process this result and respond with a VALID JSON object.".to_string(),
            agent_instructions: "You are an autonomous assistant that can call tools to solve user requests.".to_string(),
            ui_instructions: "Follow UI hydration rules for data display.".to_string(),
            language_instructions: "Detect the user's language automatically and answer using that same language.".to_string(),
            agent_max_steps_error: "agent exceeded the maximum number of tool interactions".to_string(),
            no_tools_guidance: "No additional tools are currently configured.".to_string(),
        }
    }
}

// ============================================================================
// Agent Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum tool interaction steps
    pub max_steps: u32,
    /// Verbose logging
    pub verbose: bool,
    /// Auto-execute tools
    pub auto_execute_tools: bool,
    /// Session timeout (seconds)
    pub session_timeout_secs: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            verbose: false,
            auto_execute_tools: true,
            session_timeout_secs: 300,
        }
    }
}

// ============================================================================
// Postcard Serialization
// ============================================================================

/// Configuration file path
pub const CONFIG_PATH: &str = "config/app.pc";

/// Serialize configuration to Postcard binary
pub fn config_to_postcard(config: &AppConfig) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

/// Deserialize configuration from Postcard binary
pub fn config_from_postcard(data: &[u8]) -> Result<AppConfig, String> {
    postcard::from_bytes(data)
        .map_err(|e| format!("Failed to deserialize config: {}", e))
}

/// Load configuration from file
pub fn load_config(path: Option<&Path>) -> Result<AppConfig, String> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));

    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }

    let data = std::fs::read(config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    config_from_postcard(&data)
}

/// Save configuration to file
pub fn save_config(config: &AppConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));

    // Create directory if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let data = config_to_postcard(config)?;

    std::fs::write(config_path, &data)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Initialize default configuration and save to file
pub fn init_default_config(path: Option<&Path>) -> Result<AppConfig, String> {
    let config = AppConfig::default();
    save_config(&config, path)?;
    Ok(config)
}

/// Get configuration size in bytes
pub fn config_size(path: Option<&Path>) -> Result<usize, String> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));

    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }

    std::fs::metadata(config_path)
        .map_err(|e| format!("Failed to read config metadata: {}", e))
        .map(|m| m.len() as usize)
}

// ============================================================================
// Path Helpers
// ============================================================================

/// Get the config directory path
pub fn config_dir() -> PathBuf {
    Path::new(CONFIG_PATH)
        .parent()
        .unwrap_or(Path::new("config"))
        .to_path_buf()
}

/// Get the environment file path
pub fn env_path() -> PathBuf {
    config_dir().join(".env")
}

/// Check if config file exists
pub fn config_exists() -> bool {
    Path::new(CONFIG_PATH).exists()
}
