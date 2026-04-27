use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::telemetry::TelemetryEvent;

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

/// Span status classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Ok,
    Error,
}

/// Tracing hook abstraction that can bridge into OpenTelemetry or vendor
/// tracing SDKs.
pub trait TracingHook: Send + Sync {
    fn on_span_start(&self, span: TraceSpanContext);
    fn on_span_end(&self, span: TraceSpanContext, status: TraceStatus);
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
}
