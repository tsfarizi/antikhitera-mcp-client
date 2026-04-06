//! # Antikythera SDK
//!
//! High-level API wrapper with FFI and WASM bindings for the MCP client.
//!
//! ## Feature Flags
//!
//! - `wasm` - Enable WASM bindings (enabled by default)
//! - `ffi` - Enable FFI support for C bindings
//! - `single-agent` - Single agent support (default)
//! - `multi-agent` - Multi-agent orchestration support
//! - `cloud` - Cloud integrations (GCP)
//! - `wasm-sandbox` - WASM sandboxed tool execution
//! - `full` - All features (large binary, not recommended for WASM)
//!
//! ## Examples
//!
//! ### Minimal WASM build (single agent only)
//! ```bash
//! cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release
//! ```
//!
//! ### Multi-agent WASM build
//! ```bash
//! cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release --no-default-features --features wasm,multi-agent
//! ```
//!
//! ### FFI build (native library)
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

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub mod wasm_prompt;

// FFI support (optional)
#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "ffi")]
pub use ffi::{
    mcp_server_create, mcp_server_create_with_cors, mcp_server_is_running,
    mcp_server_stop, mcp_server_stop_all, mcp_server_chat, mcp_server_get_tools,
    mcp_server_get_config, mcp_server_reload, mcp_server_update_config,
    mcp_response_add_field, mcp_response_remove_field, mcp_response_get_fields,
    mcp_response_clear_fields, mcp_response_apply_fields, mcp_response_field_count,
    mcp_set_output_format, mcp_get_output_format, mcp_format_response,
    mcp_format_final_message, mcp_extract_final_content, mcp_extract_final_data,
    mcp_extract_final_metadata, mcp_is_final_message,
    mcp_last_error, mcp_clear_error, mcp_string_free, mcp_version,
    mcp_server_count, mcp_server_list,
};

// WASM Component Model support
#[cfg(feature = "component")]
pub mod component;

#[cfg(feature = "component")]
pub use component::{PromptManager, McpClient, PromptConfig, ChatRequest, ChatResponse, AgentOptions, AgentOutcome};

// WASM configuration binary format
#[cfg(feature = "wasm-config")]
pub mod wasm_config;

#[cfg(feature = "wasm-config")]
pub use wasm_config::{
    WasmConfig, ClientSection, ModelSection, PromptSection, AgentSection,
    ProviderConfig, ServerConfig, ToolConfig, RestServerConfig,
    config_to_binary_simple, config_from_binary_simple,
    config_size_breakdown, config_summary,
};

pub mod high_level_api;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the SDK (call once at startup)
pub fn init() {
    #[cfg(feature = "wasm")]
    wasm::init();
    
    #[cfg(not(feature = "wasm"))]
    console_println!("Antikythera SDK v{} initialized", VERSION);
}

#[cfg(not(feature = "wasm"))]
macro_rules! console_println {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), $($arg)*);
        let _ = std::io::stdout().flush();
    }};
}
