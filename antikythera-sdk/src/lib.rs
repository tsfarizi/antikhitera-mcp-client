//! # Antikythera SDK
//!
//! Server-side WASM component framework for the MCP client.

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
    AgentProfile, AgentRegistry, AgentRole, AgentTask, BudgetSnapshot, CancellationToken,
    ContextId, ErrorKind, MemoryConfig, OrchestratorBudget, PipelineResult, RetryCondition,
    RoutingDecision, SyncMemoryProvider, TaskExecutionMetadata, TaskResult, TaskRetryPolicy,
};

// ============================================================================
// Vertical Slice Features
// ============================================================================

/// Agent orchestration helpers
#[cfg(feature = "multi-agent")]
pub mod agents;

#[cfg(feature = "multi-agent")]
pub use agents::{
    OrchestratorMonitorSnapshot, OrchestratorOptions, TaskResultDetail, with_hardening_runtime,
};

/// Prompt Management feature slice
#[cfg(feature = "sdk-core")]
pub mod prompts;

#[cfg(feature = "sdk-core")]
pub use prompts::{
    mcp_get_all_prompts, mcp_get_template, mcp_get_tool_guidance, mcp_reset_template,
    mcp_update_template, mcp_update_tool_guidance,
};

/// Binary Configuration feature slice (Postcard)
#[cfg(feature = "sdk-core")]
pub mod config;

#[cfg(feature = "sdk-core")]
pub use config::{
    CONFIG_PATH as POSTCARD_CONFIG_PATH, config_from_postcard, config_to_postcard,
    load_config as load_postcard_config, save_config as save_postcard_config,
};

/// Session Management module
#[cfg(feature = "sdk-core")]
pub mod session;

#[cfg(feature = "sdk-core")]
pub use session::{
    BatchExport, BatchLogExport, Message, MessageRole, Session, SessionExport, SessionLogExport,
    SessionSummary,
};

/// SDK Logging module
pub mod sdk_logging;

pub use sdk_logging::{
    ConfigFfiLogger, clear_sdk_loggers, clear_sdk_session_logs, get_latest_sdk_logs,
    get_sdk_logger, get_sdk_logs_json, query_sdk_logs, subscribe_sdk_logs,
};

/// Shared FFI helper utilities
pub mod ffi_helpers;

/// WASM Agent Module (processes LLM responses from host)
#[cfg(feature = "component")]
pub mod wasm_agent;

#[cfg(feature = "component")]
pub use wasm_agent::{
    AgentAction, AgentMessage, AgentRunnerError, AgentState, ContextPolicy, ContextSummary,
    PromptVariables, SloSnapshot, StreamEvent, StreamEventKind, TelemetrySnapshot, ToolCall,
    ToolDefinition, ToolParameterSchema, ToolRegistry, ToolResult, ToolValidationError,
    TruncationStrategy, WasmAgentConfig, append_llm_chunk, build_llm_messages, build_system_prompt,
    commit_llm_response, commit_llm_stream, drain_events, get_slo_snapshot,
    get_state as get_agent_state, get_telemetry_snapshot, get_tools_prompt,
    init as init_agent_runner, prepare_user_turn, process_llm_response,
    process_llm_response_for_session, process_tool_result, process_tool_result_for_session,
    register_tools, reset_session as reset_agent_session, set_context_policy, sweep_idle_sessions,
    validate_json_schema, validate_tool_call,
};

#[cfg(feature = "sdk-core")]
pub use antikythera_core::{HostModelClient, HostModelResponse, HostModelTransport};

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
