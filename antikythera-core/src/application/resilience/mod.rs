//! # Runtime Resilience
//!
//! This module provides retry, timeout, context-window management, and health
//! tracking primitives for the Antikythera agent runtime.
//!
//! ## Submodules
//!
//! | Submodule          | Contents                                        |
//! |--------------------|-------------------------------------------------|
//! | [`policy`]         | [`RetryPolicy`], [`TimeoutPolicy`], [`ResilienceConfig`] |
//! | [`retry`]          | [`with_retry`], [`with_retry_if`]               |
//! | [`context_window`] | [`TokenEstimator`], [`ContextWindowPolicy`], [`ContextWindowManager`], [`prune_messages`] |
//! | [`health`]         | [`HealthStatus`], [`ComponentHealth`], [`HealthTracker`] |
//! | [`observability`]  | [`CorrelationContext`], [`ComponentMetrics`], [`MetricsTracker`] |
//!
//! ## WIT / FFI surface
//!
//! [`ResilienceManager`] mirrors the WIT `resilience` interface exported by
//! the WASM component, providing JSON-serialised in/out for every operation.
//! Host runtimes that embed the WASM component call these methods to configure
//! resilience behaviour and to read back component health at runtime.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use antikythera_core::resilience::{ResilienceManager, RetryPolicy, with_retry_if};
//!
//! // Create a manager with default policies
//! let mut mgr = ResilienceManager::new();
//!
//! // Override retry policy via JSON (mirrors WIT set-config call)
//! mgr.set_config_from_json(r#"{"retry":{"max_attempts":5}}"#).unwrap();
//!
//! // Record call outcomes so health is tracked
//! mgr.health_mut().record_success("gemini-flash", 320);
//!
//! // Query health for forwarding to the host
//! let health_json = mgr.get_health_json();
//! ```

pub mod context_window;
pub mod health;
pub mod observability;
pub mod policy;
pub mod retry;

pub use context_window::{
    prune_messages, summarize_and_prune_messages, summarize_messages,
    ContextPolicyOverride, ContextWindowManager, ContextWindowPolicy, TokenEstimator,
};
pub use health::{ComponentHealth, HealthStatus, HealthTracker};
pub use observability::{ComponentMetrics, CorrelationContext, MetricsTracker};
pub use policy::{ResilienceConfig, RetryPolicy, TimeoutPolicy};
pub use retry::{with_retry, with_retry_if};

use serde_json;

// ── ResilienceManager ─────────────────────────────────────────────────────────

/// Unified facade that owns a [`ResilienceConfig`] and a [`HealthTracker`].
///
/// Every public method maps 1-to-1 to a function in the WIT `resilience`
/// interface so host languages can call them via the WASM component boundary
/// without additional glue code.
#[derive(Debug, Default)]
pub struct ResilienceManager {
    config: ResilienceConfig,
    health: HealthTracker,
    metrics: MetricsTracker,
    context: CorrelationContext,
}

impl ResilienceManager {
    /// Create a manager with default policies.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a manager with a specific [`ResilienceConfig`].
    pub fn with_config(config: ResilienceConfig) -> Self {
        Self {
            config,
            health: HealthTracker::new(),
            metrics: MetricsTracker::new(),
            context: CorrelationContext::default(),
        }
    }

    // ── Config accessors ─────────────────────────────────────────────────

    pub fn config(&self) -> &ResilienceConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: ResilienceConfig) {
        self.config = config;
    }

    // ── Health accessors ─────────────────────────────────────────────────

    pub fn health(&self) -> &HealthTracker {
        &self.health
    }

    pub fn health_mut(&mut self) -> &mut HealthTracker {
        &mut self.health
    }

    pub fn metrics(&self) -> &MetricsTracker {
        &self.metrics
    }

    pub fn metrics_mut(&mut self) -> &mut MetricsTracker {
        &mut self.metrics
    }

    pub fn context(&self) -> &CorrelationContext {
        &self.context
    }

    pub fn set_context(&mut self, context: CorrelationContext) {
        self.context = context;
    }

    // ── WIT-compatible JSON methods ───────────────────────────────────────

    /// `resilience.get-config` — return the current config as a JSON string.
    ///
    /// The returned object has the same schema as the `ResilienceConfig` type:
    ///
    /// ```json
    /// {
    ///   "retry": {
    ///     "max_attempts": 3,
    ///     "initial_delay_ms": 200,
    ///     "max_delay_ms": 10000,
    ///     "backoff_factor": 2.0
    ///   },
    ///   "timeout": {
    ///     "llm_timeout_ms": 30000,
    ///     "tool_timeout_ms": 10000
    ///   }
    /// }
    /// ```
    pub fn get_config_json(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_else(|_| "{}".to_string())
    }

    /// `resilience.set-config` — overwrite the config from a JSON string.
    ///
    /// Accepts a **partial** JSON object; fields absent from the payload are
    /// left at their current values by merging with the existing config through
    /// a full round-trip.
    ///
    /// Returns `Ok(true)` on success or `Err(reason)` if `config_json` cannot
    /// be deserialised.
    pub fn set_config_from_json(&mut self, config_json: &str) -> Result<bool, String> {
        let config: ResilienceConfig =
            serde_json::from_str(config_json).map_err(|e| e.to_string())?;
        self.config = config;
        Ok(true)
    }

    /// `resilience.get-health` — return a JSON array of all tracked component
    /// health snapshots.
    pub fn get_health_json(&self) -> String {
        self.health.snapshot_json()
    }

    /// `resilience.reset-health` — clear all accumulated health statistics.
    pub fn reset_health(&mut self) {
        self.health.reset();
    }

    /// `resilience.get-metrics` — return a JSON array of component metrics.
    pub fn get_metrics_json(&self) -> String {
        self.metrics.snapshot_json()
    }

    /// `resilience.reset-metrics` — clear all accumulated metric samples.
    pub fn reset_metrics(&mut self) {
        self.metrics.reset();
    }

    /// `resilience.set-context` — update the active correlation/session context.
    pub fn set_context_from_json(&mut self, context_json: &str) -> Result<bool, String> {
        let context: CorrelationContext =
            serde_json::from_str(context_json).map_err(|e| e.to_string())?;
        self.context = context;
        Ok(true)
    }

    /// `resilience.record-call` — add an observability sample for a component.
    pub fn record_call(
        &mut self,
        component_id: &str,
        latency_ms: u32,
        success: bool,
        error_message: Option<&str>,
    ) {
        self.metrics.record_call(
            component_id,
            latency_ms as u64,
            success,
            error_message.map(str::to_string),
            &self.context,
        );
        if success {
            self.health.record_success(component_id, latency_ms as u64);
        } else {
            self.health
                .record_failure(component_id, error_message.unwrap_or("unknown error"));
        }
    }

    /// `resilience.estimate-tokens` — estimate the token count for `text`.
    pub fn estimate_tokens(text: &str) -> u32 {
        TokenEstimator::estimate_text(text) as u32
    }

    /// `resilience.prune-messages` — prune a JSON-encoded message array to fit
    /// within `max_tokens` (with `reserve_tokens` reserved for the response).
    ///
    /// Returns the pruned array as a JSON string, or an error if the input is
    /// not a valid JSON array of `ChatMessage`-compatible objects.
    pub fn prune_messages_json(
        messages_json: &str,
        max_tokens: u32,
        reserve_tokens: u32,
    ) -> Result<String, String> {
        use crate::domain::types::ChatMessage;
        let messages: Vec<ChatMessage> =
            serde_json::from_str(messages_json).map_err(|e| e.to_string())?;
        let policy = ContextWindowPolicy {
            max_tokens: max_tokens as usize,
            reserve_for_response: reserve_tokens as usize,
            min_history_messages: 2,
        };
        let pruned = prune_messages(&messages, &policy);
        serde_json::to_string(&pruned).map_err(|e| e.to_string())
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_has_default_config() {
        let mgr = ResilienceManager::new();
        let config = mgr.config();
        assert_eq!(config.retry.max_attempts, 3);
        assert_eq!(config.timeout.llm_timeout_ms, 30_000);
    }

    #[test]
    fn get_config_json_is_valid_json() {
        let mgr = ResilienceManager::new();
        let json = mgr.get_config_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("retry").is_some());
        assert!(parsed.get("timeout").is_some());
    }

    #[test]
    fn set_config_from_json_updates_policy() {
        let mut mgr = ResilienceManager::new();
        let json = r#"{
            "retry": {"max_attempts": 7, "initial_delay_ms": 100, "max_delay_ms": 5000, "backoff_factor": 1.5},
            "timeout": {"llm_timeout_ms": 60000, "tool_timeout_ms": 5000}
        }"#;
        assert!(mgr.set_config_from_json(json).unwrap());
        assert_eq!(mgr.config().retry.max_attempts, 7);
        assert_eq!(mgr.config().timeout.llm_timeout_ms, 60_000);
    }

    #[test]
    fn set_config_from_invalid_json_returns_error() {
        let mut mgr = ResilienceManager::new();
        assert!(mgr.set_config_from_json("not-json").is_err());
    }

    #[test]
    fn get_health_json_starts_as_empty_array() {
        let mgr = ResilienceManager::new();
        let json = mgr.get_health_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    #[test]
    fn reset_health_clears_tracked_components() {
        let mut mgr = ResilienceManager::new();
        mgr.health_mut().record_success("llm", 200);
        mgr.reset_health();
        let json = mgr.get_health_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    #[test]
    fn estimate_tokens_returns_positive_for_non_empty_text() {
        assert!(ResilienceManager::estimate_tokens("hello world") > 0);
    }

    #[test]
    fn prune_messages_json_roundtrips_valid_input() {
        use crate::domain::types::{ChatMessage, MessageRole};
        let messages = vec![
            ChatMessage::new(MessageRole::User, "hello"),
            ChatMessage::new(MessageRole::Assistant, "hi there"),
        ];
        let input_json = serde_json::to_string(&messages).unwrap();
        let result = ResilienceManager::prune_messages_json(&input_json, 10_000, 100);
        assert!(result.is_ok());
        let pruned: Vec<ChatMessage> = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(pruned.len(), 2);
    }

    #[test]
    fn prune_messages_json_returns_error_for_invalid_input() {
        let result = ResilienceManager::prune_messages_json("[invalid", 1000, 100);
        assert!(result.is_err());
    }

    #[test]
    fn record_call_updates_metrics_and_health() {
        let mut mgr = ResilienceManager::new();
        mgr.set_context(CorrelationContext {
            correlation_id: Some("corr-1".to_string()),
            session_id: Some("sess-1".to_string()),
        });
        mgr.record_call("llm", 180, false, Some("timeout"));

        let metrics: serde_json::Value = serde_json::from_str(&mgr.get_metrics_json()).unwrap();
        assert_eq!(metrics.as_array().unwrap().len(), 1);
        assert_eq!(metrics[0]["component_id"], "llm");
        assert_eq!(metrics[0]["correlation_id"], "corr-1");

        let health: serde_json::Value = serde_json::from_str(&mgr.get_health_json()).unwrap();
        assert_eq!(health.as_array().unwrap().len(), 1);
        assert_eq!(health[0]["component_id"], "llm");
    }

    #[test]
    fn set_context_from_json_accepts_valid_payload() {
        let mut mgr = ResilienceManager::new();
        let payload = r#"{"correlation_id":"corr-2","session_id":"sess-2"}"#;
        assert!(mgr.set_context_from_json(payload).unwrap());
        assert_eq!(mgr.context().correlation_id.as_deref(), Some("corr-2"));
    }
}
