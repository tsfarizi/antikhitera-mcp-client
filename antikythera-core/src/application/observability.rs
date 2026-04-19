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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Caller context — propagated through all framework operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CallerContext {
    /// Unique ID for this request/session (for end-to-end tracing)
    pub correlation_id: Option<String>,
    /// User ID or service principal
    pub user_id: Option<String>,
    /// Tenant or organization ID
    pub tenant_id: Option<String>,
    /// Request source (CLI, REST, gRPC, WASM, etc.)
    pub source: Option<String>,
    /// Custom metadata propagated by the host
    pub metadata: Option<HashMap<String, String>>,
}

impl CallerContext {
    /// Create a new caller context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set correlation ID for tracing.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set user ID.
    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    /// Set tenant ID.
    pub fn with_tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    /// Set request source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add custom metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.metadata.is_none() {
            self.metadata = Some(HashMap::new());
        }
        if let Some(ref mut meta) = self.metadata {
            meta.insert(key.into(), value.into());
        }
        self
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Returns a correlation ID if present, otherwise generates a deterministic
    /// fallback using timestamp-based entropy.
    pub fn ensure_correlation_id(&mut self) -> String {
        if let Some(value) = self.correlation_id.clone() {
            return value;
        }

        let generated = format!("corr-{}", now_unix_ms());
        self.correlation_id = Some(generated.clone());
        generated
    }
}

/// Telemetry event — structured observability data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Event type (e.g., "agent_step", "tool_call", "llm_request")
    pub event_type: String,
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Timestamp (Unix epoch seconds)
    pub timestamp_ms: u64,
    /// Event-specific attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl TelemetryEvent {
    /// Create a new telemetry event.
    pub fn new(
        event_type: impl Into<String>,
        correlation_id: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            correlation_id,
            session_id,
            timestamp_ms: now_unix_ms(),
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Build flat string attributes suitable for metric exporters.
    pub fn metric_attributes(&self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        attrs.insert("event_type".to_string(), self.event_type.clone());

        if let Some(correlation_id) = &self.correlation_id {
            attrs.insert("correlation_id".to_string(), correlation_id.clone());
        }
        if let Some(session_id) = &self.session_id {
            attrs.insert("session_id".to_string(), session_id.clone());
        }

        attrs
    }
}

/// Metric type emitted by host-facing exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

/// Metric record captured by observability exporters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub name: String,
    pub kind: MetricKind,
    pub value: f64,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl MetricRecord {
    /// Construct a metric record.
    pub fn new(
        name: impl Into<String>,
        kind: MetricKind,
        value: f64,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            value,
            timestamp_ms: now_unix_ms(),
            attributes,
        }
    }
}

/// Export hook for metrics backends such as Prometheus, CloudWatch, Datadog,
/// or OpenTelemetry collectors.
pub trait MetricsExporter: Send + Sync {
    fn export_metric(&self, metric: MetricRecord);

    fn export_counter(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Counter,
            value,
            attributes,
        ));
    }

    fn export_gauge(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Gauge,
            value,
            attributes,
        ));
    }

    fn export_histogram(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Histogram,
            value,
            attributes,
        ));
    }
}

/// In-memory metric exporter used by tests and embedded hosts.
#[derive(Debug, Clone, Default)]
pub struct InMemoryMetricsExporter {
    metrics: Arc<Mutex<Vec<MetricRecord>>>,
}

impl InMemoryMetricsExporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<MetricRecord> {
        self.metrics.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.metrics.lock().unwrap().clear();
    }
}

impl MetricsExporter for InMemoryMetricsExporter {
    fn export_metric(&self, metric: MetricRecord) {
        self.metrics.lock().unwrap().push(metric);
    }
}

/// SLA latency summary values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencySummary {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl Default for LatencySummary {
    fn default() -> Self {
        Self {
            count: 0,
            min_ms: 0.0,
            max_ms: 0.0,
            avg_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
        }
    }
}

/// In-memory latency tracker with percentile summary helpers.
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    samples_ms: Vec<f64>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_ms(&mut self, duration_ms: f64) {
        if duration_ms.is_finite() && duration_ms >= 0.0 {
            self.samples_ms.push(duration_ms);
        }
    }

    pub fn count(&self) -> usize {
        self.samples_ms.len()
    }

    pub fn summary(&self) -> LatencySummary {
        if self.samples_ms.is_empty() {
            return LatencySummary::default();
        }

        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();

        LatencySummary {
            count,
            min_ms: sorted[0],
            max_ms: sorted[count - 1],
            avg_ms: sum / count as f64,
            p50_ms: percentile(&sorted, 0.50),
            p95_ms: percentile(&sorted, 0.95),
            p99_ms: percentile(&sorted, 0.99),
        }
    }
}

fn percentile(sorted_samples: &[f64], q: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }

    let q = q.clamp(0.0, 1.0);
    let index = ((sorted_samples.len() - 1) as f64 * q).round() as usize;
    sorted_samples[index]
}

/// Auditable event category for policy/tool observability trails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    PolicyDecision,
    ToolExecution,
    ModelRequest,
}

/// Structured audit record emitted by runtime checkpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub category: AuditCategory,
    pub action: String,
    pub allowed: bool,
    pub correlation_id: Option<String>,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub details: HashMap<String, String>,
}

impl AuditRecord {
    pub fn new(
        category: AuditCategory,
        action: impl Into<String>,
        allowed: bool,
        correlation_id: Option<String>,
    ) -> Self {
        Self {
            category,
            action: action.into(),
            allowed,
            correlation_id,
            timestamp_ms: now_unix_ms(),
            details: HashMap::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// In-memory audit trail store.
#[derive(Debug, Clone, Default)]
pub struct AuditTrail {
    records: Arc<Mutex<Vec<AuditRecord>>>,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, record: AuditRecord) {
        self.records.lock().unwrap().push(record);
    }

    pub fn snapshot(&self) -> Vec<AuditRecord> {
        self.records.lock().unwrap().clone()
    }

    pub fn by_category(&self, category: AuditCategory) -> Vec<AuditRecord> {
        self.snapshot()
            .into_iter()
            .filter(|record| record.category == category)
            .collect()
    }

    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }
}

/// Minimal span context used by tracing hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub correlation_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl TraceSpanContext {
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            correlation_id: None,
            name: name.into(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Tracing hook abstraction that can bridge into OpenTelemetry or vendor
/// tracing SDKs.
pub trait TracingHook: Send + Sync {
    fn on_span_start(&self, span: TraceSpanContext);
    fn on_span_end(&self, span: TraceSpanContext, status: TraceStatus);
}

/// Span status classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Ok,
    Error,
}

/// In-memory tracing hook used by tests.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTracingHook {
    started: Arc<Mutex<Vec<TraceSpanContext>>>,
    ended: Arc<Mutex<Vec<(TraceSpanContext, TraceStatus)>>>,
}

impl InMemoryTracingHook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn started_spans(&self) -> Vec<TraceSpanContext> {
        self.started.lock().unwrap().clone()
    }

    pub fn ended_spans(&self) -> Vec<(TraceSpanContext, TraceStatus)> {
        self.ended.lock().unwrap().clone()
    }
}

impl TracingHook for InMemoryTracingHook {
    fn on_span_start(&self, span: TraceSpanContext) {
        self.started.lock().unwrap().push(span);
    }

    fn on_span_end(&self, span: TraceSpanContext, status: TraceStatus) {
        self.ended.lock().unwrap().push((span, status));
    }
}

/// Observability hook — implement to receive telemetry events.
pub trait ObservabilityHook: Send + Sync {
    /// Record a telemetry event.
    fn record_event(&self, event: TelemetryEvent);

    /// Record a metric (counter, gauge, histogram).
    fn record_metric(&self, name: &str, value: f64, attributes: &HashMap<String, String>) {
        let _ = (name, value, attributes);
    }
}

/// No-op observability hook (discards all events).
pub struct NoOpObservabilityHook;

impl ObservabilityHook for NoOpObservabilityHook {
    fn record_event(&self, _event: TelemetryEvent) {}
}

/// In-memory telemetry sink for testing.
#[derive(Debug, Clone)]
pub struct InMemoryObservabilityHook {
    events: Arc<Mutex<Vec<TelemetryEvent>>>,
}

impl InMemoryObservabilityHook {
    /// Create a new in-memory hook.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get a snapshot of recorded events.
    pub fn snapshot(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Filter events by type.
    pub fn events_by_type(&self, event_type: &str) -> Vec<TelemetryEvent> {
        self.snapshot()
            .into_iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }
}

impl Default for InMemoryObservabilityHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservabilityHook for InMemoryObservabilityHook {
    fn record_event(&self, event: TelemetryEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_context_builder() {
        let ctx = CallerContext::new()
            .with_correlation_id("corr-123")
            .with_user_id("user-456")
            .with_tenant_id("tenant-789")
            .with_source("native-cli");

        assert_eq!(ctx.correlation_id, Some("corr-123".to_string()));
        assert_eq!(ctx.user_id, Some("user-456".to_string()));
        assert_eq!(ctx.tenant_id, Some("tenant-789".to_string()));
        assert_eq!(ctx.source, Some("native-cli".to_string()));
    }

    #[test]
    fn telemetry_event_serialization() {
        let event = TelemetryEvent::new(
            "agent_step",
            Some("corr-123".to_string()),
            Some("sess-456".to_string()),
        )
        .with_attribute("agent_id".to_string(), serde_json::json!("agent-001"))
        .with_attribute("step_count".to_string(), serde_json::json!(5));

        let json = event.to_json().unwrap();
        assert!(json.contains("agent_step"));
        assert!(json.contains("corr-123"));
    }

    #[test]
    fn in_memory_observability_hook() {
        let hook = InMemoryObservabilityHook::new();

        let event1 = TelemetryEvent::new("llm_request", None, Some("s1".to_string()));
        hook.record_event(event1);

        let event2 = TelemetryEvent::new("tool_call", None, Some("s1".to_string()));
        hook.record_event(event2);

        let snapshot = hook.snapshot();
        assert_eq!(snapshot.len(), 2);

        let llm_events = hook.events_by_type("llm_request");
        assert_eq!(llm_events.len(), 1);
    }

    #[test]
    fn caller_context_ensure_correlation_id_sets_value_once() {
        let mut ctx = CallerContext::new();
        let first = ctx.ensure_correlation_id();
        let second = ctx.ensure_correlation_id();

        assert_eq!(first, second);
        assert_eq!(ctx.correlation_id, Some(first));
    }

    #[test]
    fn telemetry_event_metric_attributes_contains_core_fields() {
        let event = TelemetryEvent::new(
            "tool_call",
            Some("corr-1".to_string()),
            Some("sess-1".to_string()),
        );
        let attrs = event.metric_attributes();

        assert_eq!(attrs.get("event_type"), Some(&"tool_call".to_string()));
        assert_eq!(attrs.get("correlation_id"), Some(&"corr-1".to_string()));
        assert_eq!(attrs.get("session_id"), Some(&"sess-1".to_string()));
    }

    #[test]
    fn in_memory_metrics_exporter_collects_counter_records() {
        let exporter = InMemoryMetricsExporter::new();
        exporter.export_counter("tool.calls", 3.0, HashMap::new());

        let snapshot = exporter.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "tool.calls");
        assert_eq!(snapshot[0].kind, MetricKind::Counter);
        assert_eq!(snapshot[0].value, 3.0);
    }

    #[test]
    fn metrics_exporter_clear_resets_records() {
        let exporter = InMemoryMetricsExporter::new();
        exporter.export_gauge("active.sessions", 2.0, HashMap::new());
        exporter.clear();
        assert!(exporter.snapshot().is_empty());
    }

    #[test]
    fn latency_tracker_summary_reports_percentiles() {
        let mut tracker = LatencyTracker::new();
        tracker.record_ms(10.0);
        tracker.record_ms(20.0);
        tracker.record_ms(30.0);
        tracker.record_ms(40.0);
        tracker.record_ms(50.0);

        let summary = tracker.summary();
        assert_eq!(summary.count, 5);
        assert_eq!(summary.min_ms, 10.0);
        assert_eq!(summary.max_ms, 50.0);
        assert_eq!(summary.p50_ms, 30.0);
        assert_eq!(summary.p95_ms, 50.0);
        assert_eq!(summary.p99_ms, 50.0);
    }

    #[test]
    fn latency_tracker_ignores_negative_and_nan_values() {
        let mut tracker = LatencyTracker::new();
        tracker.record_ms(-1.0);
        tracker.record_ms(f64::NAN);
        tracker.record_ms(12.0);

        let summary = tracker.summary();
        assert_eq!(summary.count, 1);
        assert_eq!(summary.p50_ms, 12.0);
    }

    #[test]
    fn audit_trail_can_filter_by_category() {
        let trail = AuditTrail::new();
        trail.append(AuditRecord::new(
            AuditCategory::PolicyDecision,
            "allow_model",
            true,
            Some("corr-1".to_string()),
        ));
        trail.append(AuditRecord::new(
            AuditCategory::ToolExecution,
            "invoke_tool",
            true,
            Some("corr-1".to_string()),
        ));

        let policies = trail.by_category(AuditCategory::PolicyDecision);
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].action, "allow_model");
    }

    #[test]
    fn audit_record_with_detail_sets_fields() {
        let record = AuditRecord::new(AuditCategory::ToolExecution, "call_weather", true, None)
            .with_detail("tool", "weather");

        assert_eq!(record.details.get("tool"), Some(&"weather".to_string()));
    }

    #[test]
    fn in_memory_tracing_hook_records_started_and_ended_spans() {
        let hook = InMemoryTracingHook::new();
        let span = TraceSpanContext::new("trace-1", "span-1", "tool_call")
            .with_correlation_id("corr-99")
            .with_parent("root-0")
            .with_attribute("tool", "search");

        hook.on_span_start(span.clone());
        hook.on_span_end(span.clone(), TraceStatus::Ok);

        let started = hook.started_spans();
        let ended = hook.ended_spans();
        assert_eq!(started.len(), 1);
        assert_eq!(ended.len(), 1);
        assert_eq!(started[0], span);
        assert_eq!(ended[0].1, TraceStatus::Ok);
    }

    #[test]
    fn percentile_returns_zero_for_empty_samples() {
        assert_eq!(percentile(&[], 0.95), 0.0);
    }
}
