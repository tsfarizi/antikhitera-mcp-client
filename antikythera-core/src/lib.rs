//! # Antikythera Core
//!
//! Core MCP protocol implementation, transport layers, and agent runtime.

pub mod application;
#[cfg(feature = "wizard")]
pub mod cli;
pub mod config;
pub mod constants;
pub mod domain;
pub mod infrastructure;

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
    AgentEvent, AgentEventStream, InMemoryStreamingResponse, StreamingMode, StreamingRequest,
    StreamingResponse, StreamingSnapshot, ToolEventPhase,
};
pub use config::AppConfig;
pub use infrastructure::model::{
    DynamicModelProvider, HostModelClient, HostModelResponse, HostModelTransport, ModelProvider,
};

// Re-export CLI argument types so binary crates don't need a direct `cli` dep.
#[cfg(feature = "wizard")]
pub use cli::{Cli, RunMode};

/// Re-export logging for easy access
pub use logging::{
    AgentLogger, ConfigLogger, ProviderLogger, TransportLogger, clear_all_loggers, clear_logs,
    get_latest_logs, get_logger, get_logs_json, logger_count, query_logs, subscribe_logs,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
