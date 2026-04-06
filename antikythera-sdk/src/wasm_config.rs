//! WASM Configuration Binary Format
//!
//! This module defines a compact, efficient binary configuration format using Postcard serialization.
//! All configuration is stored in a single binary blob with clear sections for different config types.
//!
//! ## Binary Structure
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Header (8 bytes)                                          │
//! │  ┌──────────────┬──────────────┬──────────────────────────┐ │
//! │  │ Magic (4)    │ Version (2)  │ Section Count (2)        │ │
//! │  │ 0xA7F9C3D1   │ 0x0001       │ N                        │ │
//! │  └──────────────┴──────────────┴──────────────────────────┘ │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Section Table (variable)                                   │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │ Section Entry 1: ID (1) + Offset (4) + Size (4)      │   │
//! │  │ Section Entry 2: ID (1) + Offset (4) + Size (4)      │   │
//! │  │ ...                                                    │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Section 0: Client Config                                   │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │ providers, servers, rest_server (postcard)           │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Section 1: Model Config                                    │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │ default_provider, model, tools (postcard)            │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Section 2: Prompt Config                                   │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │ template, tool_guidance, fallback, etc. (postcard)   │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Section 3: Agent Config                                    │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │ max_steps, timeout, verbose, etc. (postcard)         │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Section IDs
//!
//! - `0x01` - Client Config (providers, servers, REST settings)
//! - `0x02` - Model Config (default provider, model, tools)
//! - `0x03` - Prompt Config (all prompt templates)
//! - `0x04` - Agent Config (agent behavior settings)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Constants
// ============================================================================

/// Magic number for Antikythera config binary
pub const CONFIG_MAGIC: u32 = 0xA7F9_C3D1;

/// Current binary format version
pub const CONFIG_VERSION: u16 = 0x0001;

/// Section ID: Client Configuration
pub const SECTION_CLIENT: u8 = 0x01;

/// Section ID: Model Configuration
pub const SECTION_MODEL: u8 = 0x02;

/// Section ID: Prompt Configuration
pub const SECTION_PROMPT: u8 = 0x03;

/// Section ID: Agent Configuration
pub const SECTION_AGENT: u8 = 0x04;

// ============================================================================
// Configuration Structures
// ============================================================================

/// Complete WASM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// Client configuration (providers, servers, REST)
    pub client: ClientSection,
    /// Model configuration (default provider, model, tools)
    pub model: ModelSection,
    /// Prompt configuration (all templates)
    pub prompts: PromptSection,
    /// Agent configuration (behavior settings)
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

/// Client Configuration Section
/// Contains provider definitions, server configurations, and REST settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSection {
    /// API providers (e.g., OpenAI, Anthropic, Ollama)
    pub providers: Vec<ProviderConfig>,
    /// MCP server definitions
    pub servers: Vec<ServerConfig>,
    /// REST server settings
    pub rest_server: RestServerConfig,
    /// Environment variables for servers
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

/// Model Configuration Section
/// Contains default provider, model selection, and tool definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSection {
    /// Default provider ID
    pub default_provider: String,
    /// Default model name
    pub model: String,
    /// Available tools
    pub tools: Vec<ToolConfig>,
    /// Model-specific parameters
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

/// Prompt Configuration Section
/// Contains all prompt templates for the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSection {
    /// Main system prompt template
    pub template: String,
    /// Tool usage guidance
    pub tool_guidance: String,
    /// Fallback guidance for out-of-scope requests
    pub fallback_guidance: String,
    /// JSON retry message
    pub json_retry_message: String,
    /// Tool result formatting instruction
    pub tool_result_instruction: String,
    /// Agent behavior rules
    pub agent_instructions: String,
    /// UI hydration rules
    pub ui_instructions: String,
    /// Language detection instructions
    pub language_instructions: String,
    /// Max steps error message
    pub agent_max_steps_error: String,
    /// No tools available guidance
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

/// Agent Configuration Section
/// Contains agent behavior and performance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    /// Maximum tool calls per session
    pub max_steps: u32,
    /// Timeout in seconds
    pub timeout_secs: u32,
    /// Enable verbose logging
    pub verbose: bool,
    /// Enable tool auto-execution
    pub auto_execute_tools: bool,
    /// Session ID
    pub session_id: Option<String>,
    /// Custom agent metadata
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

// ============================================================================
// Supporting Types
// ============================================================================

/// API Provider Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub id: String,
    /// Provider type (openai, anthropic, ollama, etc.)
    pub provider_type: String,
    /// Base URL
    pub base_url: String,
    /// API key (optional)
    pub api_key: Option<String>,
    /// Additional headers
    pub headers: HashMap<String, String>,
}

/// MCP Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// Command to run
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
}

/// REST Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestServerConfig {
    /// Bind address
    pub bind: String,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// Enable API docs
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

/// Tool Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Source server
    pub server: String,
    /// Tool parameters schema (JSON)
    pub schema: Option<String>,
}

// ============================================================================
// Binary Format Structures
// ============================================================================

/// Binary header
#[derive(Debug, Clone, Copy)]
pub struct ConfigHeader {
    pub magic: u32,
    pub version: u16,
    pub section_count: u16,
}

impl ConfigHeader {
    /// Create new header
    pub fn new(section_count: u16) -> Self {
        Self {
            magic: CONFIG_MAGIC,
            version: CONFIG_VERSION,
            section_count,
        }
    }

    /// Validate header
    pub fn validate(&self) -> Result<(), String> {
        if self.magic != CONFIG_MAGIC {
            return Err(format!("Invalid magic: expected 0x{:08X}, got 0x{:08X}", CONFIG_MAGIC, self.magic));
        }
        if self.version != CONFIG_VERSION {
            return Err(format!("Unsupported version: expected {}, got {}", CONFIG_VERSION, self.version));
        }
        Ok(())
    }
}

/// Section table entry
#[derive(Debug, Clone, Copy)]
pub struct SectionEntry {
    pub section_id: u8,
    pub offset: u32,
    pub size: u32,
}

// ============================================================================
// Serialization/Deserialization
// ============================================================================

/// Serialize configuration to postcard binary
pub fn config_to_binary(config: &WasmConfig) -> Result<Vec<u8>, String> {
    // Serialize each section
    let client_data = postcard::to_allocvec(&config.client)
        .map_err(|e| format!("Failed to serialize client config: {}", e))?;
    
    let model_data = postcard::to_allocvec(&config.model)
        .map_err(|e| format!("Failed to serialize model config: {}", e))?;
    
    let prompt_data = postcard::to_allocvec(&config.prompts)
        .map_err(|e| format!("Failed to serialize prompt config: {}", e))?;
    
    let agent_data = postcard::to_allocvec(&config.agent)
        .map_err(|e| format!("Failed to serialize agent config: {}", e))?;

    // Calculate offsets
    let header_size = 8; // magic(4) + version(2) + count(2)
    let section_table_size = 4 * 9; // section_id(1) + offset(4) + size(4) = 9 bytes per section
    let section_table_start = header_size;
    
    let sections = [
        (SECTION_CLIENT, &client_data),
        (SECTION_MODEL, &model_data),
        (SECTION_PROMPT, &prompt_data),
        (SECTION_AGENT, &agent_data),
    ];

    // Build binary blob
    let mut binary = Vec::new();
    
    // Write header
    binary.extend_from_slice(&CONFIG_MAGIC.to_le_bytes());
    binary.extend_from_slice(&CONFIG_VERSION.to_le_bytes());
    binary.extend_from_slice(&(sections.len() as u16).to_le_bytes());
    
    // Calculate section offsets (after header + section table)
    let mut data_start = section_table_start + section_table_size;
    
    // Write section table (placeholder - we'll fill this in)
    for (i, (section_id, data)) in sections.iter().enumerate() {
        let offset = data_start + sections.iter().take(i).map(|(_, d)| d.len()).sum::<usize>() as u32;
        let size = data.len() as u32;
        
        binary.push(*section_id);
        binary.extend_from_slice(&offset.to_le_bytes());
        binary.extend_from_slice(&size.to_le_bytes());
    }
    
    // Write section data
    binary.extend_from_slice(&client_data);
    binary.extend_from_slice(&model_data);
    binary.extend_from_slice(&prompt_data);
    binary.extend_from_slice(&agent_data);
    
    Ok(binary)
}

/// Deserialize configuration from postcard binary
pub fn config_from_binary(data: &[u8]) -> Result<WasmConfig, String> {
    if data.len() < 8 {
        return Err("Data too short for header".to_string());
    }

    // Parse header
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    
    if magic != CONFIG_MAGIC {
        return Err(format!("Invalid magic: expected 0x{:08X}, got 0x{:08X}", CONFIG_MAGIC, magic));
    }

    // Use postcard to deserialize directly from the full data
    // This is simpler and more reliable
    let config: WasmConfig = postcard::from_bytes(data)
        .map_err(|e| format!("Failed to deserialize config: {}", e))?;
    
    Ok(config)
}

/// Serialize configuration to postcard binary (simplified version)
pub fn config_to_binary_simple(config: &WasmConfig) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

/// Deserialize configuration from postcard binary (simplified version)
pub fn config_from_binary_simple(data: &[u8]) -> Result<WasmConfig, String> {
    postcard::from_bytes(data)
        .map_err(|e| format!("Failed to deserialize config: {}", e))
}

// ============================================================================
// Utility Functions
// ============================================================================

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
         ├─ Providers: {}\n\
         ├─ Servers: {}\n\
         ├─ Tools: {}\n\
         ├─ Prompt sections: 10\n\
         ├─ Max agent steps: {}\n\
         └─ Binary size breakdown:\n\
            ├─ Client: {} bytes\n\
            ├─ Model: {} bytes\n\
            ├─ Prompts: {} bytes\n\
            ├─ Agent: {} bytes\n\
            └─ Total: {} bytes",
        config.client.providers.len(),
        config.client.servers.len(),
        config.model.tools.len(),
        config.agent.max_steps,
        sizes.get("client").unwrap_or(&0),
        sizes.get("model").unwrap_or(&0),
        sizes.get("prompts").unwrap_or(&0),
        sizes.get("agent").unwrap_or(&0),
        total,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = WasmConfig::default();
        
        // Serialize
        let binary = config_to_binary_simple(&config).expect("Failed to serialize");
        
        // Deserialize
        let loaded = config_from_binary_simple(&binary).expect("Failed to deserialize");
        
        // Verify
        assert_eq!(config.client.providers.len(), loaded.client.providers.len());
        assert_eq!(config.model.default_provider, loaded.model.default_provider);
        assert_eq!(config.prompts.template, loaded.prompts.template);
        assert_eq!(config.agent.max_steps, loaded.agent.max_steps);
    }

    #[test]
    fn test_config_with_data() {
        let mut config = WasmConfig::default();
        
        // Add some data
        config.client.providers.push(ProviderConfig {
            id: "openai".to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            headers: HashMap::new(),
        });
        
        config.model.default_provider = "openai".to_string();
        config.model.model = "gpt-4".to_string();
        
        config.prompts.template = "Custom template".to_string();
        
        config.agent.max_steps = 20;
        
        // Serialize and deserialize
        let binary = config_to_binary_simple(&config).expect("Failed to serialize");
        let loaded = config_from_binary_simple(&binary).expect("Failed to deserialize");
        
        // Verify
        assert_eq!(loaded.client.providers.len(), 1);
        assert_eq!(loaded.model.default_provider, "openai");
        assert_eq!(loaded.model.model, "gpt-4");
        assert_eq!(loaded.prompts.template, "Custom template");
        assert_eq!(loaded.agent.max_steps, 20);
    }

    #[test]
    fn test_config_size_breakdown() {
        let config = WasmConfig::default();
        let sizes = config_size_breakdown(&config);
        
        assert!(sizes.contains_key("client"));
        assert!(sizes.contains_key("model"));
        assert!(sizes.contains_key("prompts"));
        assert!(sizes.contains_key("agent"));
        
        let total: usize = sizes.values().sum();
        assert!(total > 0);
    }

    #[test]
    fn test_magic_number() {
        assert_eq!(CONFIG_MAGIC, 0xA7F9_C3D1);
        assert_eq!(CONFIG_VERSION, 0x0001);
    }
}
