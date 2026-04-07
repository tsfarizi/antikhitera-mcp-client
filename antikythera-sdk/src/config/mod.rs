//! Binary Configuration Feature Slice
//!
//! Provides postcard-based binary serialization for WASM configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WASM configuration sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    pub client: ClientSection,
    pub model: ModelSection,
    pub prompts: PromptSection,
    pub agent: AgentSection,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            client: ClientSection::default(),
            model: ModelSection::default(),
            prompts: PromptSection::default(),
            agent: AgentSection::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSection {
    pub providers: Vec<ProviderConfig>,
    pub servers: Vec<ServerConfig>,
    pub rest_server: RestServerConfig,
    pub env_vars: HashMap<String, String>,
}

impl Default for ClientSection {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            servers: Vec::new(),
            rest_server: RestServerConfig::default(),
            env_vars: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSection {
    pub default_provider: String,
    pub model: String,
    pub tools: Vec<ToolConfig>,
    pub model_params: HashMap<String, String>,
}

impl Default for ModelSection {
    fn default() -> Self {
        Self {
            default_provider: "ollama".to_string(),
            model: "llama3".to_string(),
            tools: Vec::new(),
            model_params: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSection {
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

impl Default for PromptSection {
    fn default() -> Self {
        Self {
            template: "You are a helpful AI assistant.\n\n{{custom_instruction}}\n\n{{language_guidance}}\n\n{{tool_guidance}}".to_string(),
            tool_guidance: "You have access to the following tools. Use them only when necessary.".to_string(),
            fallback_guidance: "If the request is outside the scope of available tools, apologize politely.".to_string(),
            json_retry_message: "System Error: Invalid JSON format. Please output ONLY valid JSON.".to_string(),
            tool_result_instruction: "Process tool result and respond with valid JSON.".to_string(),
            agent_instructions: "You are an autonomous assistant that can call tools.".to_string(),
            ui_instructions: "Follow UI hydration rules for data display.".to_string(),
            language_instructions: "Detect and use user's language.".to_string(),
            agent_max_steps_error: "Agent exceeded maximum steps.".to_string(),
            no_tools_guidance: "No tools currently available.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    pub max_steps: u32,
    pub timeout_secs: u32,
    pub verbose: bool,
    pub auto_execute_tools: bool,
    pub session_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            max_steps: 10,
            timeout_secs: 60,
            verbose: false,
            auto_execute_tools: true,
            session_id: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestServerConfig {
    pub bind: String,
    pub cors_origins: Vec<String>,
    pub enable_docs: bool,
}

impl Default for RestServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".to_string(),
            cors_origins: Vec::new(),
            enable_docs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub server: String,
    pub schema: Option<String>,
}

/// Serialize configuration to postcard binary
pub fn config_to_binary(config: &WasmConfig) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

/// Deserialize configuration from postcard binary
pub fn config_from_binary(data: &[u8]) -> Result<WasmConfig, String> {
    postcard::from_bytes(data)
        .map_err(|e| format!("Failed to deserialize config: {}", e))
}

/// Get configuration size breakdown
pub fn config_size_breakdown(config: &WasmConfig) -> HashMap<String, usize> {
    let mut sizes = HashMap::new();
    if let Ok(data) = postcard::to_allocvec(&config.client) {
        sizes.insert("client".to_string(), data.len());
    }
    if let Ok(data) = postcard::to_allocvec(&config.model) {
        sizes.insert("model".to_string(), data.len());
    }
    if let Ok(data) = postcard::to_allocvec(&config.prompts) {
        sizes.insert("prompts".to_string(), data.len());
    }
    if let Ok(data) = postcard::to_allocvec(&config.agent) {
        sizes.insert("agent".to_string(), data.len());
    }
    sizes
}

/// Print configuration summary
pub fn config_summary(config: &WasmConfig) -> String {
    let sizes = config_size_breakdown(config);
    let total: usize = sizes.values().sum();
    format!(
        "WASM Configuration:\n\
         Providers: {}\n\
         Servers: {}\n\
         Tools: {}\n\
         Max agent steps: {}\n\
         Binary size: {} bytes",
        config.client.providers.len(),
        config.client.servers.len(),
        config.model.tools.len(),
        config.agent.max_steps,
        total,
    )
}
