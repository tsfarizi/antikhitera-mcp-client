//! # Antikythera SDK
//!
//! High-level API wrapper with FFI and WASM bindings for the MCP client.
//!
//! ## Architecture
//!
//! This SDK follows **Vertical Slice Architecture (VSA)** where each feature
//! is organized as a self-contained slice with types, logic, and FFI bindings.
//!
//! ```text
//! src/
//! ├── client/        - MCP Client (WASM bindings)
//! ├── prompts/       - Prompt Template Management
//! ├── servers/       - MCP Server Management
//! ├── agents/        - Multi-Agent Management
//! ├── response/      - Response Formatting (JSON/Markdown/Text)
//! ├── config/        - Binary Configuration (Postcard)
//! ├── component/     - WASM Component (Host Imports)
//! └── high_level_api.rs - Native Rust API
//! ```
//!
//! ## Feature Flags
//!
//! - `wasm` - Enable WASM bindings (enabled by default)
//! - `ffi` - Enable FFI support for C bindings
//! - `single-agent` - Single agent support (default)
//! - `multi-agent` - Multi-agent orchestration support
//!
//! ## Examples
//!
//! ### WASM build
//! ```bash
//! cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release
//! ```
//!
//! ### FFI build
//! ```bash
//! cargo build -p antikythera-sdk --release --features ffi
//! ```

// Re-export core types (always available)
pub use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome};
pub use antikythera_core::application::client::{ClientConfig, McpClient};
pub use antikythera_core::config::AppConfig;

// Conditional exports based on features
#[cfg(feature = "multi-agent")]
pub use antikythera_core::application::agent::multi_agent::{
    AgentRegistry, AgentProfile, AgentRole, MemoryProvider, MemoryConfig, ContextId,
};

// ============================================================================
// Vertical Slice Features
// ============================================================================

/// MCP Client feature slice (WASM bindings)
#[cfg(feature = "wasm")]
pub mod client;

#[cfg(feature = "wasm")]
pub use client::{WasmClient, init as wasm_init};

/// Prompt Management feature slice
pub mod prompts;

pub use prompts::{
    mcp_get_template, mcp_update_template, mcp_reset_template,
    mcp_get_tool_guidance, mcp_update_tool_guidance,
    mcp_get_all_prompts, mcp_get_raw_config,
};

/// Server Management feature slice
pub mod servers;

pub use servers::{
    // Types
    McpServerConfig, ServerTransport, ServerValidationResult,
    ServerStatus, ServerOperationResult,
    // FFI
    mcp_add_server, mcp_remove_server, mcp_list_servers,
    mcp_get_server, mcp_validate_server, mcp_export_servers_config,
    mcp_import_servers_config,
};

/// Agent Management feature slice
pub mod agents;

pub use agents::{
    // Types
    AgentConfig, AgentType, SkillLevel, AgentCapability,
    AgentValidationResult, AgentStatus, AgentTaskRequest,
    AgentTaskResult, OrchestrationResult,
    // FFI
    mcp_register_agent, mcp_unregister_agent, mcp_list_agents,
    mcp_get_agent, mcp_get_agent_status, mcp_validate_agent,
    mcp_export_agents_config, mcp_import_agents_config,
};

/// Response Formatting feature slice
pub mod response;

pub use response::{
    OutputFormat,
    mcp_set_output_format, mcp_get_output_format, mcp_format_response,
};

/// Binary Configuration feature slice
pub mod config;

pub use config::{
    WasmConfig, ClientSection, ModelSection, PromptSection, AgentSection,
    ProviderConfig, ServerConfig, RestServerConfig, ToolConfig,
    config_to_binary, config_from_binary,
    config_size_breakdown, config_summary,
};

/// WASM Component feature slice (Host Imports)
pub mod component;

pub use component::{
    // Host Import Types
    LlmRequest, LlmResponse, ToolCallEvent, ToolExecutionResult, LogEvent,
    HostImports, DelegatingAgent,
    // FFI
    run_agent_with_host,
};

// ============================================================================
// Legacy Modules
// ============================================================================

/// Native high-level API wrapper
pub mod high_level_api;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the SDK
pub fn init() {
    #[cfg(feature = "wasm")]
    wasm_init();

    #[cfg(not(feature = "wasm"))]
    {
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), "Antikythera SDK v{} initialized", VERSION);
        let _ = std::io::stdout().flush();
    }
}
