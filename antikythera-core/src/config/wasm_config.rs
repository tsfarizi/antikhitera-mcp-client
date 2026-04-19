//! WASM Agent Configuration
//!
//! Minimal config that WASM needs - NO provider info.
//! Providers and API keys are managed by host runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// WASM Agent Configuration
// ============================================================================

/// Configuration for WASM agent (minimal, no provider info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum tool interaction steps
    pub max_steps: u32,
    /// Verbose logging
    pub verbose: bool,
    /// Auto-execute tools without confirmation
    pub auto_execute_tools: bool,
    /// Session timeout (seconds)
    pub session_timeout_secs: u32,
    /// Session ID
    pub session_id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            verbose: false,
            auto_execute_tools: true,
            session_timeout_secs: 300,
            session_id: format!("session-{}", chrono::Utc::now().timestamp_millis()),
        }
    }
}

// ============================================================================
// Prompt Configuration
// ============================================================================

/// Prompt templates for agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Main system prompt template
    pub template: String,
    /// Tool usage guidance
    pub tool_guidance: String,
    /// Fallback guidance when request is outside tool scope
    pub fallback_guidance: String,
    /// JSON retry message
    pub json_retry_message: String,
    /// Tool result instruction
    pub tool_result_instruction: String,
    /// Agent instructions
    pub agent_instructions: String,
    /// UI hydration instructions
    pub ui_instructions: String,
    /// Language detection instructions
    pub language_instructions: String,
    /// Agent max steps error
    pub agent_max_steps_error: String,
    /// No tools guidance
    pub no_tools_guidance: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            template: Self::default_template().to_string(),
            tool_guidance: "You have access to tools. Use them only when necessary.".to_string(),
            fallback_guidance: "If request is outside tool scope, apologize politely.".to_string(),
            json_retry_message: "Invalid JSON. Output ONLY valid JSON object.".to_string(),
            tool_result_instruction: "Tool complete. Process result and respond with valid JSON."
                .to_string(),
            agent_instructions: "You are an autonomous assistant that calls tools.".to_string(),
            ui_instructions: "Follow UI hydration rules for data display.".to_string(),
            language_instructions: "Detect user language and respond in same language.".to_string(),
            agent_max_steps_error: "Agent exceeded maximum tool interactions.".to_string(),
            no_tools_guidance: "No tools available.".to_string(),
        }
    }
}

impl PromptConfig {
    /// Default prompt template
    pub fn default_template() -> &'static str {
        "You are a helpful AI assistant.\n\n{{custom_instruction}}\n\n{{language_guidance}}\n\n{{tool_guidance}}"
    }
}

// ============================================================================
// JSON Schema Configuration
// ============================================================================

/// JSON schema definition for response validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaConfig {
    /// Schema name
    pub name: String,
    /// Schema definition (JSON encoded)
    pub schema_json: String,
    /// Whether to enforce validation
    pub enforced: bool,
}

// ============================================================================
// Complete WASM Config
// ============================================================================

/// Complete WASM agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAgentConfig {
    /// Agent settings
    pub agent: AgentConfig,
    /// Prompt templates
    pub prompts: PromptConfig,
    /// JSON schemas for validation
    pub schemas: Vec<JsonSchemaConfig>,
    /// Custom key-value pairs
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Default for WasmAgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            prompts: PromptConfig::default(),
            schemas: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

// ============================================================================
// Serialization
// ============================================================================

/// WASM config file path
pub const WASM_CONFIG_PATH: &str = "wasm-agent.pc";

/// Serialize config to Postcard
pub fn to_postcard(config: &WasmAgentConfig) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(config).map_err(|e| format!("Serialize error: {}", e))
}

/// Deserialize config from Postcard
pub fn from_postcard(data: &[u8]) -> Result<WasmAgentConfig, String> {
    postcard::from_bytes(data).map_err(|e| format!("Deserialize error: {}", e))
}

/// Load config from file
pub fn load(path: Option<&Path>) -> Result<WasmAgentConfig, String> {
    let config_path = path.unwrap_or(Path::new(WASM_CONFIG_PATH));
    if !config_path.exists() {
        return Err(format!("Config not found: {}", config_path.display()));
    }
    let data = std::fs::read(config_path).map_err(|e| format!("Read error: {}", e))?;
    from_postcard(&data)
}

/// Save config to file
pub fn save(config: &WasmAgentConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = path.unwrap_or(Path::new(WASM_CONFIG_PATH));
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create dir error: {}", e))?;
    }
    let data = to_postcard(config)?;
    std::fs::write(config_path, data).map_err(|e| format!("Write error: {}", e))
}

/// Check if config exists
pub fn exists() -> bool {
    Path::new(WASM_CONFIG_PATH).exists()
}
