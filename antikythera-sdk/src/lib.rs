//! # Antikythera SDK
//!
//! Server-side WASM component framework for the MCP client.
//!
//! ## WASM Target
//!
//! This framework targets **server-side WASM** (WASI component model, `wasm32-wasip1`).
//! The compiled `.wasm` binary is hosted by a native process (Rust, Python, Go, etc.)
//! that embeds `wasmtime` and calls exports via the WIT interface.
//! The host process handles all external I/O (LLM calls, tool execution, persistence)
//! through host imports declared in `wit/antikythera.wit`.
//!
//! Build targets:
//! - **Server-side WASM component**: `cargo component build --target wasm32-wasip1`
//! - **Native Rust** (CLI, tests, embedding): `cargo build`
//!
//! ## Architecture
//!
//! The SDK is organized as a set of modules that support both native and server-side WASM builds.
//!
//! ```text
//! src/
//! ├── component/     - Server-side WASM Component (Host Imports/Exports via WIT)
//! ├── wasm_agent/    - WASM Agent FSM and LLM response processing
//! ├── config/        - Binary Configuration (Postcard)
//! ├── session/       - Session Management and History
//! ├── prompts/       - Prompt Template Management
//! ├── response/      - Response Formatting
//! └── high_level_api.rs - Native Rust API (native builds only)
//! ```
//!
//! ## Feature Flags
//!
//! - `component` - Server-side WASM component model (primary WASM target)
//! - `single-agent` - Single agent support (default)
//! - `multi-agent` - Multi-agent orchestration support
//!
//! ## Examples
//!
//! ### Server-side WASM component build
//! ```bash
//! cargo component build --target wasm32-wasip1 --release
//! ```

// Re-export core types
#[cfg(feature = "sdk-core")]
pub use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome};
#[cfg(feature = "sdk-core")]
pub use antikythera_core::application::client::{ClientConfig, McpClient};
#[cfg(feature = "sdk-core")]
pub use antikythera_core::config::AppConfig;

// Conditional exports based on features
#[cfg(all(feature = "sdk-core", feature = "multi-agent"))]
pub use antikythera_core::application::agent::multi_agent::{
    AgentProfile,
    AgentRegistry,
    AgentRole,
    // task types
    AgentTask,
    // hardening — monitoring
    BudgetSnapshot,
    // hardening — manipulation
    CancellationToken,
    ContextId,
    ErrorKind,
    MemoryConfig,
    MemoryProvider,
    OrchestratorBudget,
    PipelineResult,
    RetryCondition,
    RoutingDecision,
    TaskExecutionMetadata,
    TaskResult,
    TaskRetryPolicy,
};

// ============================================================================
// Vertical Slice Features
// ============================================================================

/// Prompt Management feature slice
/// Agent Management feature slice (config, validation, FFI, orchestration helpers)
pub mod agents;

#[cfg(feature = "multi-agent")]
pub use agents::{OrchestratorMonitorSnapshot, OrchestratorOptions, TaskResultDetail};
pub use agents::{StreamingModeOption, StreamingOptions};

/// Prompt Management feature slice
#[cfg(feature = "sdk-core")]
pub mod prompts;

#[cfg(feature = "sdk-core")]
pub use prompts::{
    mcp_get_all_prompts, mcp_get_template, mcp_get_tool_guidance, mcp_reset_template,
    mcp_update_template, mcp_update_tool_guidance,
};

/// Response Formatting feature slice
pub mod response;

pub use response::{mcp_format_response, mcp_get_output_format, mcp_set_output_format};

/// Binary Configuration feature slice (Postcard)
#[cfg(feature = "sdk-core")]
pub mod config;

#[cfg(feature = "sdk-core")]
pub use config::{
    CONFIG_PATH as POSTCARD_CONFIG_PATH,
    config_exists as postcard_config_exists,
    config_from_postcard,
    config_size as postcard_config_size,
    // Postcard operations
    config_to_postcard,
    init_default_config as init_default_postcard_config,
    load_config as load_postcard_config,
    save_config as save_postcard_config,
};

/// JSON Schema Validation (enforce JSON output format)
pub mod json_schema;

pub use json_schema::{
    // Types
    JsonSchema,
    // Validator
    JsonValidator,
    RetryManager,
    ValidationError,
};

/// Session Management module
#[cfg(feature = "sdk-core")]
pub mod session;

#[cfg(feature = "sdk-core")]
pub use session::{
    BatchExport,
    BatchLogExport,
    // Types
    Message,
    MessageRole,
    // Manager
    SdkSessionManager,
    Session,
    SessionExport,
    SessionLogExport,
    SessionSummary,
};

/// SDK Logging module
pub mod sdk_logging;

pub use sdk_logging::{
    AgentLogger,
    // Module loggers
    ConfigFfiLogger,
    PromptLogger,
    ResponseLogger,
    ServerLogger,
    WasmAgentLogger,
    clear_sdk_loggers,
    clear_sdk_session_logs,
    get_latest_sdk_logs,
    // Global functions
    get_sdk_logger,
    get_sdk_logs_json,
    // Query API
    query_sdk_logs,
    subscribe_sdk_logs,
};

/// WASM Agent Module (processes LLM responses from host)
#[cfg(feature = "component")]
pub mod wasm_agent;

#[cfg(feature = "component")]
pub use wasm_agent::{
    // Types
    AgentAction,
    AgentMessage,
    AgentState,
    ContextPolicy,
    ContextSummary,
    PromptVariables,
    StreamEvent,
    StreamEventKind,
    TelemetrySnapshot,
    ToolCall,
    ToolResult,
    TruncationStrategy,
    WasmAgentConfig,
    // Runner contract
    append_llm_chunk,
    build_llm_messages,
    build_system_prompt,
    commit_llm_response,
    commit_llm_stream,
    drain_events,
    get_state as get_agent_state,
    get_telemetry_snapshot,
    init as init_agent_runner,
    prepare_user_turn,
    // Processor
    process_llm_response,
    process_llm_response_for_session,
    process_tool_result,
    process_tool_result_for_session,
    reset_session as reset_agent_session,
    set_context_policy,
    validate_json_schema,
};

/// WASM Component feature slice (Host Imports/Exports)
#[cfg(feature = "component")]
pub mod component;

#[cfg(feature = "component")]
pub use component::{
    DelegatingAgent,
    HostImports,
    // Host Import Types
    LlmRequest,
    LlmResponse,
    LogEvent,
    ToolCallEvent,
    ToolExecutionResult,
    // Host functions
    run_agent_with_host,
};

// ============================================================================
// Legacy Modules
// ============================================================================

/// Native high-level API wrapper
#[cfg(feature = "sdk-core")]
pub mod high_level_api;

#[cfg(feature = "sdk-core")]
pub use antikythera_core::{HostModelClient, HostModelResponse, HostModelTransport};

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
