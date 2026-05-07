//! Policy Audit Events
//!
//! Structured audit trail for policy decisions, failures, and overrides.
//! Exported as JSON-serializable events for integration with host observability systems.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::logging::ResilienceLogger;

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
        let policy_name: String = policy_name.into();
        let decision: String = decision.into();
        let reason: String = reason.into();

        let log = ResilienceLogger::new(&crate::logging::get_active_session());
        match &event_type {
            PolicyEventType::ToolAccessDenied | PolicyEventType::HealthCheckFailed => {
                log.warn(format!(
                    "Policy audit: {:?} | policy={} decision={} reason={}",
                    event_type, policy_name, decision, reason
                ));
            }
            _ => {
                log.info(format!(
                    "Policy audit: {:?} | policy={} decision={} reason={}",
                    event_type, policy_name, decision, reason
                ));
            }
        }

        Self {
            timestamp: Utc::now().to_rfc3339(),
            correlation_id,
            session_id,
            event_type,
            policy_name,
            decision,
            reason,
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
        self.events
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|e| {
                let log = ResilienceLogger::new(&crate::logging::get_active_session());
                log.warn(format!(
                    "InMemoryAuditSink events lock poisoned in snapshot: {}",
                    e
                ));
                Vec::new()
            })
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        match self.events.lock() {
            Ok(mut guard) => guard.clear(),
            Err(e) => {
                let log = ResilienceLogger::new(&crate::logging::get_active_session());
                log.warn(format!(
                    "InMemoryAuditSink events lock poisoned in clear: {}",
                    e
                ));
            }
        }
    }
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyAuditSink for InMemoryAuditSink {
    fn record_event(&self, event: PolicyAuditEvent) {
        match self.events.lock() {
            Ok(mut guard) => guard.push(event),
            Err(e) => {
                let log = ResilienceLogger::new(&crate::logging::get_active_session());
                log.warn(format!(
                    "InMemoryAuditSink events lock poisoned in record_event: {}",
                    e
                ));
            }
        }
    }
}
