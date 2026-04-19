//! Observability Hooks
//!
//! Framework primitives for propagating caller context, correlation IDs, and
//! structured telemetry events through the agent runtime.
//!
//! These hooks allow embedding hosts to instrument the framework and correlate
//! framework events with their own observability systems (logging, tracing, metrics).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Caller context — propagated through all framework operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}

/// Telemetry event — structured observability data.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
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
    events: std::sync::Arc<std::sync::Mutex<Vec<TelemetryEvent>>>,
}

impl InMemoryObservabilityHook {
    /// Create a new in-memory hook.
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
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
        let event = TelemetryEvent::new("agent_step", Some("corr-123".to_string()), Some("sess-456".to_string()))
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
}
