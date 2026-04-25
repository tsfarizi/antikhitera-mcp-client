//! # Antikythera Core
//!
//! Core MCP protocol implementation, transport layers, and agent runtime.

pub mod application;
pub mod config;
pub mod constants;
pub mod domain;
pub mod infrastructure;

/// Security module for input validation, rate limiting, and secrets management
pub mod security;

/// Unified logging system for all core operations
pub mod logging;

// Re-export commonly used types
pub use application::agent::{Agent, AgentOptions, AgentOutcome, ToolDescriptor};

// Re-export resilience module at crate root
pub use application::client::{ChatRequest, ChatResult, ClientConfig, McpClient, PreparedChatTurn};
pub use application::hooks::{
    AuthHook, CorrelationHook, HookContext, HookError, HookOperation, HookRegistry,
    HostHookMiddleware, InMemoryTelemetryHook, PolicyDecision, PolicyDecisionHook,
    PolicyDecisionInput, PolicyTarget, TelemetryHook,
};
pub use application::observability::{
    AuditCategory, AuditRecord, AuditTrail, CallerContext, InMemoryMetricsExporter,
    InMemoryObservabilityHook, InMemoryTracingHook, LatencySummary, LatencyTracker, MetricKind,
    MetricRecord, MetricsExporter, NoOpObservabilityHook, ObservabilityHook, TelemetryEvent,
    TraceSpanContext, TraceStatus, TracingHook,
};
pub use application::resilience;
pub use application::resilience::{
    ComponentHealth, ContextWindowPolicy, HealthStatus, HealthTracker, ResilienceConfig,
    ResilienceManager, RetryPolicy, TimeoutPolicy, TokenEstimator, prune_messages, with_retry,
    with_retry_if,
};
pub use application::streaming::{
    AgentEvent, AgentEventStream, BufferPolicy, ClientInputStream, InMemoryStreamingResponse,
    StreamingBuffer, StreamingMode, StreamingPhase2Options, StreamingRequest, StreamingResponse,
    StreamingSnapshot, ToolEventPhase,
};
pub mod streaming {
    pub use crate::application::streaming::*;
}
pub use config::AppConfig;
pub use infrastructure::model::{
    DynamicModelProvider, HostModelClient, HostModelResponse, HostModelTransport, ModelProvider,
};

/// Re-export log entry types used by the TUI tracing bridge.
pub use antikythera_log::LogLevel;
/// Re-export logging for easy access
pub use logging::{
    AgentLogger, ConfigLogger, ProviderLogger, TransportLogger, clear_all_loggers, clear_logs,
    get_active_session, get_latest_logs, get_logger, get_logs_json, logger_count, query_logs,
    set_active_session, subscribe_logs,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
