//! # Application Module
//!
//! This module contains the core application logic for the MCP client.
//!
//! ## Submodules
//!
//! - [`client`] - The main MCP client for communicating with AI models
//! - [`agent`] - Autonomous agent that can use tools and execute multi-step tasks
//! - [`context_management`] - Message history and context window management
//! - [`discovery`] - Auto-discovery and loading of MCP servers from a folder
//! - [`hooks`] - Host authentication, correlation, policy, and telemetry middleware
//! - [`stdio`] - Standard input/output interface for command-line interaction
//! - [`streaming`] - Token/event streaming primitives and host adapters
//! - [`tooling`] - Tool server management and MCP server integration
//! - [`ui`] - Schema-driven UI assembler for dynamic component layout
//! - [`resilience`] - Retry, timeout, context management, and health tracking
//! - [`observability`] - Caller context, telemetry events, and tracing hooks

pub mod agent;
pub mod client;
pub mod context_management;
pub mod discovery;
pub mod hooks;
pub mod observability;
pub mod resilience;
pub mod services;
#[cfg(feature = "native-transport")]
pub mod stdio;
pub mod streaming;
pub mod tooling;
pub mod ui;

pub use hooks::{
    AuthHook, CorrelationHook, HookContext, HookError, HookOperation, HookRegistry,
    HostHookMiddleware, InMemoryTelemetryHook, PolicyDecision, PolicyDecisionHook,
    PolicyDecisionInput, PolicyTarget, TelemetryHook,
};
pub use observability::{
    AuditCategory, AuditRecord, AuditTrail, CallerContext, InMemoryMetricsExporter,
    InMemoryObservabilityHook, InMemoryTracingHook, LatencySummary, LatencyTracker, MetricKind,
    MetricRecord, MetricsExporter, NoOpObservabilityHook, ObservabilityHook, TelemetryEvent,
    TraceSpanContext, TraceStatus, TracingHook,
};
pub use streaming::{
    AgentEvent, AgentEventStream, BufferPolicy, ClientInputStream, InMemoryStreamingResponse,
    StreamingBuffer, StreamingMode, StreamingPhase2Options, StreamingRequest, StreamingResponse,
    StreamingSnapshot, ToolEventPhase,
};
