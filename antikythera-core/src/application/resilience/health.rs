//! Runtime health tracking.
//!
//! Tracks per-component health based on success/failure call counts and a
//! weighted moving-average latency.  The status is exposed as a
//! JSON-serialisable snapshot for forwarding to hosts via the WIT
//! `resilience` interface.
//!
//! # Status thresholds
//!
//! | Error rate        | [`HealthStatus`]            |
//! |-------------------|-----------------------------|
//! | 0 %               | [`HealthStatus::Healthy`]   |
//! | > 0 % and < 50 %  | [`HealthStatus::Degraded`]  |
//! | ≥ 50 %            | [`HealthStatus::Unhealthy`] |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Status ────────────────────────────────────────────────────────────────────

/// Coarse health classification for a tracked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// No recent errors; operating normally.
    Healthy,
    /// Non-zero error rate but still functional (error rate < 50 %).
    Degraded,
    /// Half or more of recent calls failed; treat as unavailable.
    Unhealthy,
}

impl HealthStatus {
    fn from_error_rate(rate: f64) -> Self {
        if rate == 0.0 {
            HealthStatus::Healthy
        } else if rate < 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

// ── Per-component health ──────────────────────────────────────────────────────

/// Accumulated health metrics for a single named component (e.g. an LLM
/// provider ID or a tool name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub component_id: String,
    pub status: HealthStatus,
    pub total_calls: u64,
    pub successful_calls: u64,
    /// Fraction of calls that resulted in an error (`0.0` … `1.0`).
    pub error_rate: f64,
    /// Exponential moving average of successful call latency in milliseconds.
    pub avg_latency_ms: f64,
    pub last_error: Option<String>,
}

impl ComponentHealth {
    fn new(component_id: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            status: HealthStatus::Healthy,
            total_calls: 0,
            successful_calls: 0,
            error_rate: 0.0,
            avg_latency_ms: 0.0,
            last_error: None,
        }
    }

    /// Record a successful call with `latency_ms` round-trip time.
    pub fn record_success(&mut self, latency_ms: u64) {
        self.total_calls += 1;
        self.successful_calls += 1;
        self.avg_latency_ms = ema(self.avg_latency_ms, latency_ms as f64, self.total_calls);
        self.error_rate = 1.0 - (self.successful_calls as f64 / self.total_calls as f64);
        self.status = HealthStatus::from_error_rate(self.error_rate);
    }

    /// Record a failed call with the error description.
    pub fn record_failure(&mut self, error: impl Into<String>) {
        self.total_calls += 1;
        self.last_error = Some(error.into());
        self.error_rate = 1.0 - (self.successful_calls as f64 / self.total_calls as f64);
        self.status = HealthStatus::from_error_rate(self.error_rate);
    }
}

/// Exponential moving average with alpha = 2 / (min(N, 20) + 1).
///
/// Capping N at 20 gives recent samples a persistent influence instead of the
/// weight approaching zero as N grows very large.
fn ema(current: f64, new_sample: f64, n: u64) -> f64 {
    if n <= 1 {
        return new_sample;
    }
    let alpha = 2.0 / (n.min(20) as f64 + 1.0);
    current * (1.0 - alpha) + new_sample * alpha
}

// ── Tracker ───────────────────────────────────────────────────────────────────

/// Aggregates health metrics across multiple named components.
///
/// Typical usage is to hold one `HealthTracker` per agent session and call
/// [`record_success`][HealthTracker::record_success] /
/// [`record_failure`][HealthTracker::record_failure] around every LLM and
/// tool call.  The host can query the snapshot via the WIT `resilience`
/// interface using [`snapshot_json`][HealthTracker::snapshot_json].
#[derive(Debug, Default)]
pub struct HealthTracker {
    components: HashMap<String, ComponentHealth>,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful invocation of `component_id` with the measured
    /// latency.
    pub fn record_success(&mut self, component_id: &str, latency_ms: u64) {
        self.components
            .entry(component_id.to_string())
            .or_insert_with(|| ComponentHealth::new(component_id))
            .record_success(latency_ms);
    }

    /// Record a failed invocation of `component_id`.
    pub fn record_failure(&mut self, component_id: &str, error: impl Into<String>) {
        self.components
            .entry(component_id.to_string())
            .or_insert_with(|| ComponentHealth::new(component_id))
            .record_failure(error);
    }

    /// Retrieve the health snapshot for a specific component, if any calls
    /// have been recorded for it.
    pub fn health_of(&self, component_id: &str) -> Option<&ComponentHealth> {
        self.components.get(component_id)
    }

    /// Aggregate status: the worst status across all tracked components.
    ///
    /// Returns [`HealthStatus::Healthy`] when no components have been
    /// recorded yet.
    pub fn overall_status(&self) -> HealthStatus {
        let mut worst = HealthStatus::Healthy;
        for c in self.components.values() {
            match c.status {
                HealthStatus::Unhealthy => return HealthStatus::Unhealthy,
                HealthStatus::Degraded => worst = HealthStatus::Degraded,
                HealthStatus::Healthy => {}
            }
        }
        worst
    }

    /// Reset all accumulated statistics for all components.
    pub fn reset(&mut self) {
        self.components.clear();
    }

    /// Serialise the full component map to a JSON array string.
    ///
    /// This is the format exposed through the WIT `resilience.get-health`
    /// export so that host languages (Python, Go, etc.) can render or log the
    /// health data without additional parsing.
    pub fn snapshot_json(&self) -> String {
        let snapshot: Vec<&ComponentHealth> = self.components.values().collect();
        serde_json::to_string(&snapshot).unwrap_or_else(|_| "[]".to_string())
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_component_after_first_success_is_healthy() {
        let mut tracker = HealthTracker::new();
        tracker.record_success("llm", 100);
        assert_eq!(
            tracker.health_of("llm").unwrap().status,
            HealthStatus::Healthy
        );
    }

    #[test]
    fn first_failure_alone_makes_component_unhealthy() {
        let mut tracker = HealthTracker::new();
        tracker.record_failure("llm", "connection refused");
        // 1 failure / 1 total = 100% error rate → Unhealthy
        assert_eq!(
            tracker.health_of("llm").unwrap().status,
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn failure_after_success_results_in_unhealthy_at_fifty_percent() {
        let mut tracker = HealthTracker::new();
        tracker.record_success("llm", 100);
        tracker.record_failure("llm", "network timeout");
        // 1 success + 1 failure = 50% error rate → Unhealthy (rate >= 0.5)
        assert_eq!(
            tracker.health_of("llm").unwrap().status,
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn many_successes_after_one_failure_recovers_to_degraded() {
        let mut tracker = HealthTracker::new();
        tracker.record_failure("llm", "transient err");
        for _ in 0..5 {
            tracker.record_success("llm", 50);
        }
        // 1 failure / 6 total ≈ 16.7% → Degraded
        assert_eq!(
            tracker.health_of("llm").unwrap().status,
            HealthStatus::Degraded
        );
    }

    #[test]
    fn overall_status_is_worst_component_status() {
        let mut tracker = HealthTracker::new();
        tracker.record_success("a", 10); // Healthy
        tracker.record_failure("b", "error"); // Unhealthy
        assert_eq!(tracker.overall_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn overall_status_is_healthy_with_no_components() {
        let tracker = HealthTracker::new();
        assert_eq!(tracker.overall_status(), HealthStatus::Healthy);
    }

    #[test]
    fn snapshot_json_is_a_valid_json_array() {
        let mut tracker = HealthTracker::new();
        tracker.record_success("llm", 100);
        tracker.record_failure("tools", "timeout");
        let json = tracker.snapshot_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn reset_clears_all_component_data() {
        let mut tracker = HealthTracker::new();
        tracker.record_success("llm", 100);
        tracker.reset();
        assert!(tracker.health_of("llm").is_none());
        assert_eq!(tracker.overall_status(), HealthStatus::Healthy);
    }

    #[test]
    fn health_of_unknown_component_returns_none() {
        let tracker = HealthTracker::new();
        assert!(tracker.health_of("unknown").is_none());
    }

    #[test]
    fn last_error_is_stored_on_failure() {
        let mut tracker = HealthTracker::new();
        tracker.record_failure("llm", "HTTP 503");
        let health = tracker.health_of("llm").unwrap();
        assert_eq!(health.last_error.as_deref(), Some("HTTP 503"));
    }
}
