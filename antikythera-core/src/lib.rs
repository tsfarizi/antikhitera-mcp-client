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
pub use application::resilience;
pub use application::resilience::{
    ResilienceConfig, ResilienceManager, RetryPolicy, TimeoutPolicy,
    ContextPolicyOverride, ContextWindowManager, ContextWindowPolicy,
    TokenEstimator, prune_messages, summarize_and_prune_messages, summarize_messages,
    HealthStatus, ComponentHealth, HealthTracker,
    ComponentMetrics, CorrelationContext, MetricsTracker,
    with_retry, with_retry_if,
};
pub use application::client::{ChatRequest, ClientConfig, McpClient};
pub use config::AppConfig;
pub use infrastructure::model::{
    DynamicModelProvider, ModelProvider, ModelStreamEvent, ModelToolCall,
    ModelToolChoice, ModelToolDefinition,
};

// Re-export CLI argument types so binary crates don't need a direct `cli` dep.
#[cfg(feature = "wizard")]
pub use cli::{Cli, RunMode};

/// Re-export logging for easy access
pub use logging::{
    get_logger, clear_all_loggers, logger_count,
    ConfigLogger, AgentLogger, TransportLogger, ProviderLogger,
    query_logs, get_latest_logs, get_logs_json, subscribe_logs, clear_logs,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


