//! Policy Audit Events
//!
//! Structured audit trail for policy decisions, failures, and overrides.
//! Exported as JSON-serializable events for integration with host observability systems.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy decision event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEventType {
    /// Context policy applied (threshold check, truncation, summarization)
    ContextPolicyApplied,
    /// Context policy override activated
    ContextPolicyOverride,
    /// Tool access denied by policy
    ToolAccessDenied,
    /// Tool access granted
    ToolAccessGranted,
    /// Rate limit policy triggered
    RateLimitTriggered,
    /// Timeout policy triggered
    TimeoutTriggered,
    /// Retry policy activated
    RetryPolicyActivated,
    /// Health check failed
    HealthCheckFailed,
    /// Custom policy decision
    CustomPolicy,
}

/// Audit event for policy decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAuditEvent {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Event type
    pub event_type: PolicyEventType,
    /// Policy name or ID
    pub policy_name: String,
    /// Decision: allow / deny / override
    pub decision: String,
    /// Reason for the decision
    pub reason: String,
    /// Affected resource (tool name, agent id, etc.)
    pub resource: Option<String>,
    /// Caller context (user ID, tenant, etc.)
    pub caller: Option<HashMap<String, String>>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl PolicyAuditEvent {
    /// Create a new policy audit event.
    pub fn new(
        correlation_id: Option<String>,
        session_id: Option<String>,
        event_type: PolicyEventType,
        policy_name: impl Into<String>,
        decision: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            correlation_id,
            session_id,
            event_type,
            policy_name: policy_name.into(),
            decision: decision.into(),
            reason: reason.into(),
            resource: None,
            caller: None,
            metadata: None,
        }
    }

    /// Add resource information.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Add caller context.
    pub fn with_caller(mut self, caller: HashMap<String, String>) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Policy audit event sink — implement this to capture events.
pub trait PolicyAuditSink: Send + Sync {
    /// Record a policy audit event.
    fn record_event(&self, event: PolicyAuditEvent);
}

/// No-op audit sink (discards all events).
pub struct NoOpAuditSink;

impl PolicyAuditSink for NoOpAuditSink {
    fn record_event(&self, _event: PolicyAuditEvent) {}
}

/// In-memory audit sink for testing.
#[derive(Debug, Clone)]
pub struct InMemoryAuditSink {
    events: std::sync::Arc<std::sync::Mutex<Vec<PolicyAuditEvent>>>,
}

impl InMemoryAuditSink {
    /// Create a new in-memory audit sink.
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Get a snapshot of recorded events.
    pub fn snapshot(&self) -> Vec<PolicyAuditEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyAuditSink for InMemoryAuditSink {
    fn record_event(&self, event: PolicyAuditEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_audit_event_serialization() {
        let event = PolicyAuditEvent::new(
            Some("corr-123".to_string()),
            Some("sess-456".to_string()),
            PolicyEventType::ContextPolicyApplied,
            "context_policy",
            "allow",
            "context window within limits",
        )
        .with_resource("agent-001");

        let json = event.to_json().unwrap();
        assert!(json.contains("context_policy"));
        assert!(json.contains("corr-123"));
    }

    #[test]
    fn in_memory_audit_sink() {
        let sink = InMemoryAuditSink::new();

        let event1 = PolicyAuditEvent::new(
            None,
            Some("s1".to_string()),
            PolicyEventType::ToolAccessGranted,
            "tool_policy",
            "allow",
            "tool authorized",
        );
        sink.record_event(event1);

        let event2 = PolicyAuditEvent::new(
            None,
            Some("s1".to_string()),
            PolicyEventType::ToolAccessDenied,
            "tool_policy",
            "deny",
            "tool not in allowlist",
        );
        sink.record_event(event2);

        let snapshot = sink.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].event_type, PolicyEventType::ToolAccessGranted);
        assert_eq!(snapshot[1].event_type, PolicyEventType::ToolAccessDenied);
    }
}
