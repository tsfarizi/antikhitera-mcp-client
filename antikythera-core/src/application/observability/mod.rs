//! Observability Hooks
//!
//! Framework primitives for propagating caller context, correlation IDs, and
//! structured telemetry events through the agent runtime.
//!
//! These hooks allow embedding hosts to instrument the framework and correlate
//! framework events with their own observability systems (logging, tracing, metrics).
//!
//! # Stability
//! Stable.
//!
//! # Example
//! ```
//! use antikythera_core::{
//!     CallerContext, InMemoryMetricsExporter, LatencyTracker, MetricsExporter, TelemetryEvent,
//! };
//!
//! let context = CallerContext::new().with_correlation_id("corr-001");
//! let event = TelemetryEvent::new("tool_call", context.correlation_id.clone(), None);
//! let mut tracker = LatencyTracker::new();
//! tracker.record_ms(120.0);
//! tracker.record_ms(240.0);
//! let summary = tracker.summary();
//! assert_eq!(summary.p50_ms, 240.0);
//!
//! let exporter = InMemoryMetricsExporter::default();
//! exporter.export_counter("tool.calls", 1.0, event.metric_attributes());
//! assert_eq!(exporter.snapshot().len(), 1);
//! ```

pub mod audit;
pub mod metrics;
pub mod telemetry;
pub mod tracing;

pub use audit::{AuditCategory, AuditRecord, AuditTrail};
pub use metrics::{
    InMemoryMetricsExporter, LatencySummary, LatencyTracker, MetricKind, MetricRecord,
    MetricsExporter,
};
pub use telemetry::{CallerContext, TelemetryEvent};
pub use tracing::{
    InMemoryObservabilityHook, InMemoryTracingHook, NoOpObservabilityHook, ObservabilityHook,
    TraceSpanContext, TraceStatus, TracingHook,
};

pub(crate) fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
