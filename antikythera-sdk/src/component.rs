//! WASM Component interfaces
//!
//! This module defines traits for the WASM component.
//! The build script automatically generates WIT files from these traits.
//!
//! ## Adding New Functions
//!
//! 1. Add function signature to `PromptManager` or `McpClient` trait
//! 2. Run: `cargo run --release -p build-scripts -- wit`
//! 3. WIT is auto-generated from your code!

use serde::{Deserialize, Serialize};

// ============================================================================
// Data Types
// ============================================================================

/// Prompt configuration for AI agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Main system prompt template with placeholders
    pub template: String,
    /// Guidance when tools are available
    pub tool_guidance: String,
    /// Guidance when no tools match the request
    pub fallback_guidance: String,
    /// Message sent to LLM when JSON parsing fails
    pub json_retry_message: String,
    /// Instruction for tool result formatting
    pub tool_result_instruction: String,
    /// Base autonomous assistant rules and JSON constraints
    pub agent_instructions: String,
    /// Rules for late-binding UI hydration
    pub ui_instructions: String,
    /// Instructions for language detection and adherence
    pub language_instructions: String,
    /// User-facing error message for interaction limits
    pub agent_max_steps_error: String,
    /// Guidance when no tools are available
    pub no_tools_guidance: String,
}

/// Model provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    pub id: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Chat request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
    pub raw_mode: bool,
    pub bypass_template: bool,
    pub force_json: bool,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub session_id: Option<String>,
    pub tokens_used: Option<u32>,
}

/// Agent execution options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOptions {
    pub max_steps: u32,
    pub verbose: bool,
    pub session_id: Option<String>,
}

/// Agent execution outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutcome {
    pub response: String,
    pub logs: Vec<String>,
    pub session_id: Option<String>,
    pub steps_executed: u32,
}

/// Tool descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
}

/// Tool inventory response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInventory {
    pub tools: Vec<ToolDescriptor>,
}

/// Configuration reload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Component Trait Definitions
// ============================================================================

/// Prompt template management interface
/// 
/// Add new prompt-related functions here and run:
/// `cargo run --release -p build-scripts -- wit`
pub trait PromptManager {
    /// Get the main system prompt template
    fn get_template() -> Result<String, String>;
    
    /// Update the main system prompt template
    fn update_template(template: String) -> Result<(), String>;
    
    /// Reset the main template to default
    fn reset_template() -> Result<(), String>;
    
    /// Get tool guidance prompt
    fn get_tool_guidance() -> Result<String, String>;
    
    /// Update tool guidance prompt
    fn update_tool_guidance(guidance: String) -> Result<(), String>;
    
    /// Reset tool guidance to default
    fn reset_tool_guidance() -> Result<(), String>;
    
    /// Get fallback guidance prompt
    fn get_fallback_guidance() -> Result<String, String>;
    
    /// Update fallback guidance prompt
    fn update_fallback_guidance(guidance: String) -> Result<(), String>;
    
    /// Reset fallback guidance to default
    fn reset_fallback_guidance() -> Result<(), String>;
    
    /// Get JSON retry message
    fn get_json_retry_message() -> Result<String, String>;
    
    /// Update JSON retry message
    fn update_json_retry_message(message: String) -> Result<(), String>;
    
    /// Reset JSON retry message to default
    fn reset_json_retry_message() -> Result<(), String>;
    
    /// Get tool result instruction
    fn get_tool_result_instruction() -> Result<String, String>;
    
    /// Update tool result instruction
    fn update_tool_result_instruction(instruction: String) -> Result<(), String>;
    
    /// Reset tool result instruction to default
    fn reset_tool_result_instruction() -> Result<(), String>;
    
    /// Get agent instructions
    fn get_agent_instructions() -> Result<String, String>;
    
    /// Update agent instructions
    fn update_agent_instructions(instructions: String) -> Result<(), String>;
    
    /// Reset agent instructions to default
    fn reset_agent_instructions() -> Result<(), String>;
    
    /// Get UI instructions
    fn get_ui_instructions() -> Result<String, String>;
    
    /// Update UI instructions
    fn update_ui_instructions(instructions: String) -> Result<(), String>;
    
    /// Reset UI instructions to default
    fn reset_ui_instructions() -> Result<(), String>;
    
    /// Get language instructions
    fn get_language_instructions() -> Result<String, String>;
    
    /// Update language instructions
    fn update_language_instructions(instructions: String) -> Result<(), String>;
    
    /// Reset language instructions to default
    fn reset_language_instructions() -> Result<(), String>;
    
    /// Get agent max steps error message
    fn get_agent_max_steps_error() -> Result<String, String>;
    
    /// Update agent max steps error message
    fn update_agent_max_steps_error(message: String) -> Result<(), String>;
    
    /// Reset agent max steps error to default
    fn reset_agent_max_steps_error() -> Result<(), String>;
    
    /// Get no tools guidance
    fn get_no_tools_guidance() -> Result<String, String>;
    
    /// Update no tools guidance
    fn update_no_tools_guidance(guidance: String) -> Result<(), String>;
    
    /// Reset no tools guidance to default
    fn reset_no_tools_guidance() -> Result<(), String>;
    
    /// Get all prompts as JSON object
    fn get_all_prompts() -> Result<String, String>;
    
    /// Reset all prompts to defaults
    fn reset_all_prompts() -> Result<(), String>;
    
    /// Get the raw model.toml file content
    fn get_raw_config() -> Result<String, String>;
}

/// MCP client interface for chat and agent operations
/// 
/// Add new client functions here and run:
/// `cargo run --release -p build-scripts -- wit`
pub trait McpClient {
    /// Initialize client with configuration
    fn init(config_json: String) -> Result<(), String>;
    
    /// Send a chat message
    fn chat(request: ChatRequest) -> Result<ChatResponse, String>;
    
    /// Simple chat with just prompt
    fn chat_simple(prompt: String) -> Result<String, String>;
    
    /// Run agent with autonomous tool execution
    fn run_agent(prompt: String, options: AgentOptions) -> Result<AgentOutcome, String>;
    
    /// List available tools
    fn list_tools() -> Result<ToolInventory, String>;
    
    /// Get current prompt template
    fn get_prompt_template() -> Result<String, String>;
    
    /// Reload configuration
    fn reload_config() -> Result<ReloadResponse, String>;
}
