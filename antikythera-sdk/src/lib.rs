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
    mcp_get_all_prompts,
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
    mcp_set_output_format, mcp_get_output_format, mcp_format_response,
};

/// Binary Configuration feature slice (Postcard)
pub mod config;

pub use config::{
    // Postcard operations
    config_to_postcard, config_from_postcard,
    load_config as load_postcard_config, save_config as save_postcard_config,
    init_default_config as init_default_postcard_config,
    config_size as postcard_config_size, config_exists as postcard_config_exists,
    CONFIG_PATH as POSTCARD_CONFIG_PATH,
};

/// Configuration FFI (Postcard-based)
pub mod config_ffi;

pub use config_ffi::{
    // Core config FFI
    mcp_config_init, mcp_config_exists, mcp_config_size,
    mcp_config_get_all, mcp_config_set_all,
    mcp_config_export, mcp_config_import, mcp_config_reset,
    mcp_config_use_from, mcp_config_backup_to,
    // Field-level FFI
    mcp_config_get, mcp_config_set,
    // Provider FFI
    mcp_config_add_provider, mcp_config_remove_provider, mcp_config_list_providers,
    mcp_config_set_provider_api_key, mcp_config_get_provider_api_key,
    mcp_config_add_provider_model, mcp_config_remove_provider_model,
    mcp_config_list_provider_models,
    // Prompt FFI
    mcp_config_get_prompt, mcp_config_set_prompt, mcp_config_list_prompts,
    // Agent FFI
    mcp_config_get_agent, mcp_config_set_agent_max_steps,
    mcp_config_set_agent_verbose, mcp_config_set_agent_auto_execute,
};

/// JSON Schema Validation (enforce JSON output format)
pub mod json_schema;

pub use json_schema::{
    // Types
    JsonSchema, ValidationError,
    // Validator
    JsonValidator, RetryManager,
    // FFI Functions
    ffi::mcp_json_schema_register,
    ffi::mcp_json_schema_get,
    ffi::mcp_json_schema_list,
    ffi::mcp_json_schema_remove,
    ffi::mcp_json_schema_example,
    ffi::mcp_json_validate,
    ffi::mcp_json_schema_prompt,
    ffi::mcp_json_retry_init,
    ffi::mcp_json_retry_record_error,
    ffi::mcp_json_retry_prompt,
    ffi::mcp_json_retry_is_exhausted,
};

/// Session Management module
pub mod session;

pub use session::{
    // Types
    Message, MessageRole, Session, SessionSummary,
    SessionExport, BatchExport,
    SessionLogExport, BatchLogExport,
    // Manager
    SdkSessionManager,
    // FFI - Session management
    mcp_session_create, mcp_session_get, mcp_session_list,
    mcp_session_add_message, mcp_session_get_history,
    mcp_session_export, mcp_session_import,
    mcp_session_delete, mcp_session_clear,
    mcp_batch_export, mcp_batch_import,
    // FFI - Session log integration
    mcp_session_export_logs, mcp_session_import_logs,
    mcp_session_get_logs, mcp_session_batch_export_logs,
    mcp_session_batch_import_logs,
};

/// SDK Logging module
pub mod sdk_logging;

pub use sdk_logging::{
    // Global functions
    get_sdk_logger, clear_sdk_loggers,
    // Module loggers
    ConfigFfiLogger, ServerLogger, AgentLogger, PromptLogger, ResponseLogger, WasmAgentLogger,
    // Query API
    query_sdk_logs, get_latest_sdk_logs, get_sdk_logs_json, subscribe_sdk_logs, clear_sdk_session_logs,
};

/// WASM Agent Module (processes LLM responses from host)
pub mod wasm_agent;

pub use wasm_agent::{
    // Types
    AgentAction, AgentState, WasmAgentConfig,
    AgentMessage, ToolCall, ToolResult, PromptVariables,
    // Processor
    process_llm_response,
    process_tool_result,
    build_system_prompt,
    build_llm_messages,
    validate_json_schema,
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
