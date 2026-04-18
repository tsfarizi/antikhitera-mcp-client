//! Integration tests for the resilience module.
//!
//! Verifies that the public API of `antikythera_core::resilience` works
//! correctly end-to-end from an external crate perspective, mirroring the
//! access pattern a host application would use.

use antikythera_core::resilience::{
    prune_messages, ContextWindowPolicy, HealthStatus, HealthTracker, ResilienceConfig,
    ResilienceManager, RetryPolicy, TimeoutPolicy, TokenEstimator, with_retry_if,
};
use antikythera_core::domain::types::{ChatMessage, MessageRole};

// ── RetryPolicy integration ───────────────────────────────────────────────────

#[test]
fn retry_policy_json_roundtrip_preserves_all_fields() {
    let policy = RetryPolicy {
        max_attempts: 7,
        initial_delay_ms: 500,
        max_delay_ms: 20_000,
        backoff_factor: 1.5,
    };
    let json = serde_json::to_string(&policy).unwrap();
    let parsed: RetryPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_attempts, 7);
    assert_eq!(parsed.initial_delay_ms, 500);
    assert_eq!(parsed.max_delay_ms, 20_000);
    assert!((parsed.backoff_factor - 1.5).abs() < 1e-9);
}

#[test]
fn timeout_policy_durations_match_millisecond_fields() {
    let policy = TimeoutPolicy {
        llm_timeout_ms: 45_000,
        tool_timeout_ms: 8_000,
    };
    assert_eq!(policy.llm_duration().as_secs(), 45);
    assert_eq!(policy.tool_duration().as_secs(), 8);
}

// ── with_retry_if integration ─────────────────────────────────────────────────

#[tokio::test]
async fn with_retry_if_succeeds_after_transient_network_errors() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let attempts = Arc::new(AtomicU32::new(0));
    let a = Arc::clone(&attempts);
    let policy = RetryPolicy {
        max_attempts: 5,
        initial_delay_ms: 1,
        max_delay_ms: 5,
        backoff_factor: 1.0,
    };

    let result: Result<String, String> = with_retry_if(
        &policy,
        || {
            let a = Arc::clone(&a);
            async move {
                let n = a.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("network error".to_string())
                } else {
                    Ok("success".to_string())
                }
            }
        },
        |e| e.contains("network"),
    )
    .await;

    assert_eq!(result.unwrap(), "success");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

// ── TokenEstimator integration ────────────────────────────────────────────────

#[test]
fn token_estimator_scales_proportionally_to_text_length() {
    let short = TokenEstimator::estimate_text("hello");
    let long = TokenEstimator::estimate_text(&"word ".repeat(200));
    assert!(long > short * 10, "long text should have much higher token estimate");
}

#[test]
fn token_estimator_message_slice_sums_correctly() {
    let messages = vec![
        ChatMessage::new(MessageRole::User, "What is 2+2?"),
        ChatMessage::new(MessageRole::Assistant, "The answer is 4."),
    ];
    let total = TokenEstimator::estimate_messages(&messages);
    let manual: usize = messages.iter().map(TokenEstimator::estimate_message).sum();
    assert_eq!(total, manual);
}

// ── prune_messages integration ────────────────────────────────────────────────

#[test]
fn prune_messages_with_tight_budget_keeps_newest_messages() {
    let mut messages = Vec::new();
    messages.push(ChatMessage::new(MessageRole::System, "Be helpful."));
    for i in 0..8 {
        let role = if i % 2 == 0 {
            MessageRole::User
        } else {
            MessageRole::Assistant
        };
        messages.push(ChatMessage::new(role, &format!("turn {i}")));
    }

    let policy = ContextWindowPolicy {
        max_tokens: 80,
        reserve_for_response: 20,
        min_history_messages: 2,
    };

    let pruned = prune_messages(&messages, &policy);

    // System message must survive
    assert!(pruned.iter().any(|m| m.role == MessageRole::System));
    // At least min_history_messages non-system messages
    let non_system: Vec<_> = pruned
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .collect();
    assert!(non_system.len() >= policy.min_history_messages);
    // Newest message (last in original) must be in the output
    let last_original = messages.last().unwrap();
    assert!(pruned
        .iter()
        .any(|m| m.content() == last_original.content()));
}

// ── HealthTracker integration ─────────────────────────────────────────────────

#[test]
fn health_tracker_aggregates_multiple_components() {
    let mut tracker = HealthTracker::new();
    // Component A: healthy
    for _ in 0..10 {
        tracker.record_success("llm-primary", 200);
    }
    // Component B: degraded
    tracker.record_success("tool-server", 50);
    tracker.record_failure("tool-server", "timeout");
    tracker.record_success("tool-server", 60);
    tracker.record_success("tool-server", 55);

    let primary = tracker.health_of("llm-primary").unwrap();
    assert_eq!(primary.status, HealthStatus::Healthy);

    let tool = tracker.health_of("tool-server").unwrap();
    assert_ne!(tool.status, HealthStatus::Healthy); // degraded or worse

    // Overall: worst component wins
    assert_ne!(tracker.overall_status(), HealthStatus::Healthy);
}

#[test]
fn health_tracker_snapshot_json_contains_all_tracked_components() {
    let mut tracker = HealthTracker::new();
    tracker.record_success("llm", 100);
    tracker.record_success("tools", 50);
    tracker.record_success("cache", 5);

    let json = tracker.snapshot_json();
    let arr: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(arr.len(), 3);
    let ids: Vec<&str> = arr
        .iter()
        .map(|v| v["component_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"llm"));
    assert!(ids.contains(&"tools"));
    assert!(ids.contains(&"cache"));
}

// ── ResilienceManager integration ─────────────────────────────────────────────

#[test]
fn resilience_manager_full_lifecycle() {
    let mut mgr = ResilienceManager::new();

    // Default config
    assert_eq!(mgr.config().retry.max_attempts, 3);

    // Set config via JSON
    let new_config = ResilienceConfig {
        retry: RetryPolicy {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 5_000,
            backoff_factor: 2.0,
        },
        timeout: TimeoutPolicy {
            llm_timeout_ms: 20_000,
            tool_timeout_ms: 5_000,
        },
    };
    let config_json = serde_json::to_string(&new_config).unwrap();
    assert!(mgr.set_config_from_json(&config_json).unwrap());
    assert_eq!(mgr.config().retry.max_attempts, 5);

    // Record health
    mgr.health_mut().record_success("llm", 150);
    mgr.health_mut().record_failure("llm", "timeout");

    let health_json = mgr.get_health_json();
    let arr: Vec<serde_json::Value> = serde_json::from_str(&health_json).unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["component_id"].as_str().unwrap(), "llm");

    // Reset and verify clean slate
    mgr.reset_health();
    let after_reset: Vec<serde_json::Value> =
        serde_json::from_str(&mgr.get_health_json()).unwrap();
    assert!(after_reset.is_empty());
}

#[test]
fn resilience_manager_estimate_tokens_is_consistent() {
    let t1 = ResilienceManager::estimate_tokens("hello");
    let t2 = ResilienceManager::estimate_tokens("hello");
    assert_eq!(t1, t2, "token estimation must be deterministic");
    assert!(t1 > 0);
}

#[test]
fn resilience_manager_prune_messages_json_handles_empty_array() {
    let result = ResilienceManager::prune_messages_json("[]", 1000, 100);
    assert!(result.is_ok());
    let pruned: Vec<serde_json::Value> = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(pruned.is_empty());
}
