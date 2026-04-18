//! Observability primitives for correlation context and lightweight metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrelationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_id: String,
    pub total_calls: u64,
    pub failed_calls: u64,
    pub avg_latency_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl ComponentMetrics {
    fn new(component_id: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            total_calls: 0,
            failed_calls: 0,
            avg_latency_ms: 0.0,
            last_latency_ms: None,
            last_error: None,
            correlation_id: None,
            session_id: None,
        }
    }

    fn record(
        &mut self,
        latency_ms: u64,
        success: bool,
        error_message: Option<String>,
        context: &CorrelationContext,
    ) {
        self.total_calls += 1;
        if !success {
            self.failed_calls += 1;
        }
        let sample_count = self.total_calls.min(20) as f64;
        if self.avg_latency_ms == 0.0 {
            self.avg_latency_ms = latency_ms as f64;
        } else {
            let alpha = 2.0 / (sample_count + 1.0);
            self.avg_latency_ms = alpha * latency_ms as f64 + (1.0 - alpha) * self.avg_latency_ms;
        }
        self.last_latency_ms = Some(latency_ms);
        self.last_error = error_message;
        self.correlation_id = context.correlation_id.clone();
        self.session_id = context.session_id.clone();
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricsTracker {
    components: HashMap<String, ComponentMetrics>,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_call(
        &mut self,
        component_id: &str,
        latency_ms: u64,
        success: bool,
        error_message: Option<String>,
        context: &CorrelationContext,
    ) {
        let metrics = self
            .components
            .entry(component_id.to_string())
            .or_insert_with(|| ComponentMetrics::new(component_id));
        metrics.record(latency_ms, success, error_message, context);
    }

    pub fn snapshot(&self) -> Vec<ComponentMetrics> {
        let mut metrics: Vec<ComponentMetrics> = self.components.values().cloned().collect();
        metrics.sort_by(|left, right| left.component_id.cmp(&right.component_id));
        metrics
    }

    pub fn snapshot_json(&self) -> String {
        serde_json::to_string(&self.snapshot()).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn reset(&mut self) {
        self.components.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_tracker_records_context_and_error_state() {
        let mut tracker = MetricsTracker::new();
        let context = CorrelationContext {
            correlation_id: Some("corr-1".to_string()),
            session_id: Some("sess-1".to_string()),
        };

        tracker.record_call("llm", 250, false, Some("timeout".to_string()), &context);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].component_id, "llm");
        assert_eq!(snapshot[0].failed_calls, 1);
        assert_eq!(snapshot[0].correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(snapshot[0].session_id.as_deref(), Some("sess-1"));
        assert_eq!(snapshot[0].last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn metrics_tracker_reset_clears_snapshot() {
        let mut tracker = MetricsTracker::new();
        tracker.record_call("tools", 40, true, None, &CorrelationContext::default());
        tracker.reset();
        assert!(tracker.snapshot().is_empty());
    }
}
