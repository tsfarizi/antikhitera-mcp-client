use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        let generated = format!("corr-{}", super::now_unix_ms());
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
            timestamp_ms: super::now_unix_ms(),
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
    }
}
