//! Resilience policy configuration types.
//!
//! Defines [`RetryPolicy`], [`TimeoutPolicy`], and [`ResilienceConfig`] which
//! govern retry back-off and call-timeout behaviour for LLM and tool calls.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Retry policy ──────────────────────────────────────────────────────────────

/// Controls how many times a failing call is retried and how long to wait
/// between each attempt (exponential back-off).
///
/// # Default values
///
/// | Field              | Default  |
/// |--------------------|----------|
/// | `max_attempts`     | 3        |
/// | `initial_delay_ms` | 200 ms   |
/// | `max_delay_ms`     | 10 000 ms |
/// | `backoff_factor`   | 2.0      |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of total attempts (including the first).
    /// A value of `1` disables retries.
    pub max_attempts: u32,
    /// Base delay in milliseconds before the first retry.
    pub initial_delay_ms: u64,
    /// Hard cap on the computed delay in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier applied to the delay on each successive attempt.
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 200,
            max_delay_ms: 10_000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Return the wait duration before attempt number `attempt` (0-based).
    ///
    /// For `attempt = 0` the result is `initial_delay_ms`; each subsequent
    /// attempt multiplies by `backoff_factor`, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let ms =
            (self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32)) as u64;
        Duration::from_millis(ms.min(self.max_delay_ms))
    }

    /// A policy with `max_attempts = 1` (no retries at all).
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Default::default()
        }
    }

    /// Five attempts with the default back-off curve.
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            ..Default::default()
        }
    }
}

// ── Timeout policy ────────────────────────────────────────────────────────────

/// Per-call time limits for LLM inference and tool execution.
///
/// # Default values
///
/// | Field             | Default   |
/// |-------------------|-----------|
/// | `llm_timeout_ms`  | 30 000 ms |
/// | `tool_timeout_ms` | 10 000 ms |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// Maximum milliseconds to wait for the LLM to return a response.
    pub llm_timeout_ms: u64,
    /// Maximum milliseconds to wait for a single MCP tool call to complete.
    pub tool_timeout_ms: u64,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            llm_timeout_ms: 30_000,
            tool_timeout_ms: 10_000,
        }
    }
}

impl TimeoutPolicy {
    pub fn llm_duration(&self) -> Duration {
        Duration::from_millis(self.llm_timeout_ms)
    }

    pub fn tool_duration(&self) -> Duration {
        Duration::from_millis(self.tool_timeout_ms)
    }
}

// ── Combined config ───────────────────────────────────────────────────────────

/// Top-level resilience configuration bundling retry and timeout policies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResilienceConfig {
    pub retry: RetryPolicy,
    pub timeout: TimeoutPolicy,
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_for_attempt_zero_equals_initial_delay() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(200));
    }

    #[test]
    fn delay_doubles_each_attempt_with_factor_two() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(800));
    }

    #[test]
    fn delay_is_capped_at_max_delay() {
        let policy = RetryPolicy {
            initial_delay_ms: 5_000,
            max_delay_ms: 7_000,
            backoff_factor: 3.0,
            ..Default::default()
        };
        // 5000 * 3 = 15000 > 7000 → should be capped at 7000
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(7_000));
    }

    #[test]
    fn no_retry_policy_has_exactly_one_attempt() {
        assert_eq!(RetryPolicy::no_retry().max_attempts, 1);
    }

    #[test]
    fn aggressive_policy_has_five_attempts() {
        assert_eq!(RetryPolicy::aggressive().max_attempts, 5);
    }

    #[test]
    fn timeout_policy_duration_helpers_match_ms_fields() {
        let policy = TimeoutPolicy {
            llm_timeout_ms: 5_000,
            tool_timeout_ms: 2_000,
        };
        assert_eq!(policy.llm_duration(), Duration::from_secs(5));
        assert_eq!(policy.tool_duration(), Duration::from_secs(2));
    }

    #[test]
    fn resilience_config_roundtrips_json() {
        let config = ResilienceConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ResilienceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.retry.max_attempts, config.retry.max_attempts);
        assert_eq!(
            parsed.timeout.llm_timeout_ms,
            config.timeout.llm_timeout_ms
        );
    }
}
