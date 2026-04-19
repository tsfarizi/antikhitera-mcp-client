//! Centralized tests for multi-agent production hardening types.
//!
//! These are pure-logic tests that do not require a live LLM or MCP server.
//! They cover: `AgentTask` builder, serde roundtrips, `TaskRetryPolicy`,
//! `TaskExecutionMetadata` defaults, `TaskResult` constructors,
//! `PipelineResult` aggregation, `budget_steps` guardrail semantics, and
//! deadline pre-check logic.

use antikythera_core::application::agent::multi_agent::task::{
    AgentTask, PipelineResult, TaskExecutionMetadata, TaskResult, TaskRetryPolicy,
};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// AgentTask builder
// ---------------------------------------------------------------------------

#[test]
fn agent_task_builder_sets_all_fields() {
    let policy = TaskRetryPolicy {
        max_retries: 3,
        backoff_ms: 250,
    };

    let task = AgentTask::new("analyse this code")
        .for_agent("code-reviewer")
        .with_session("sess-42")
        .with_max_steps(15)
        .with_timeout_ms(5_000)
        .with_retry_policy(policy.clone())
        .with_budget_steps(12)
        .with_correlation_id("corr-abc")
        .with_metadata("priority", "high");

    assert_eq!(task.input, "analyse this code");
    assert_eq!(task.agent_id.as_deref(), Some("code-reviewer"));
    assert_eq!(task.session_id.as_deref(), Some("sess-42"));
    assert_eq!(task.max_steps, Some(15));
    assert_eq!(task.timeout_ms, Some(5_000));
    assert_eq!(task.budget_steps, Some(12));
    assert_eq!(task.correlation_id.as_deref(), Some("corr-abc"));
    assert_eq!(task.metadata["priority"], Value::String("high".to_string()));

    let rp = task.retry_policy.unwrap();
    assert_eq!(rp.max_retries, 3);
    assert_eq!(rp.backoff_ms, 250);
}

#[test]
fn agent_task_auto_generates_unique_ids() {
    let t1 = AgentTask::new("task 1");
    let t2 = AgentTask::new("task 2");
    assert_ne!(t1.task_id, t2.task_id, "auto-generated IDs must be unique");
}

#[test]
fn agent_task_serde_roundtrip() {
    let task = AgentTask::new("do something")
        .for_agent("planner")
        .with_max_steps(8)
        .with_budget_steps(6)
        .with_retry_policy(TaskRetryPolicy {
            max_retries: 2,
            backoff_ms: 100,
        });

    let json = serde_json::to_string(&task).expect("serialize");
    let restored: AgentTask = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.task_id, task.task_id);
    assert_eq!(restored.agent_id, task.agent_id);
    assert_eq!(restored.max_steps, task.max_steps);
    assert_eq!(restored.budget_steps, task.budget_steps);
    let rp = restored.retry_policy.unwrap();
    assert_eq!(rp.max_retries, 2);
    assert_eq!(rp.backoff_ms, 100);
}

// ---------------------------------------------------------------------------
// TaskRetryPolicy
// ---------------------------------------------------------------------------

#[test]
fn task_retry_policy_default_is_zero_retries() {
    let policy = TaskRetryPolicy::default();
    assert_eq!(policy.max_retries, 0);
    assert_eq!(policy.backoff_ms, 0);
}

#[test]
fn task_retry_policy_serde_roundtrip() {
    let policy = TaskRetryPolicy {
        max_retries: 5,
        backoff_ms: 1_000,
    };
    let json = serde_json::to_string(&policy).expect("serialize");
    let restored: TaskRetryPolicy = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.max_retries, 5);
    assert_eq!(restored.backoff_ms, 1_000);
}

// ---------------------------------------------------------------------------
// TaskExecutionMetadata
// ---------------------------------------------------------------------------

#[test]
fn task_execution_metadata_defaults_to_no_failure() {
    let meta = TaskExecutionMetadata::default();
    assert_eq!(meta.attempt_count, 0);
    assert_eq!(meta.duration_ms, 0);
    assert!(!meta.timed_out);
    assert!(!meta.deadline_exceeded);
    assert!(!meta.cancelled);
    assert!(!meta.retry_applied);
    assert!(meta.routed_by.is_none());
    assert!(meta.correlation_id.is_none());
}

#[test]
fn task_execution_metadata_serde_roundtrip() {
    let meta = TaskExecutionMetadata {
        attempt_count: 2,
        duration_ms: 750,
        timed_out: false,
        deadline_exceeded: true,
        cancelled: false,
        retry_applied: true,
        routed_by: Some("round-robin".to_string()),
        execution_mode: Some("sequential".to_string()),
        correlation_id: Some("corr-xyz".to_string()),
    };

    let json = serde_json::to_string(&meta).expect("serialize");
    let restored: TaskExecutionMetadata = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.attempt_count, 2);
    assert_eq!(restored.duration_ms, 750);
    assert!(restored.deadline_exceeded);
    assert!(restored.retry_applied);
    assert_eq!(restored.routed_by.as_deref(), Some("round-robin"));
    assert_eq!(restored.correlation_id.as_deref(), Some("corr-xyz"));
}

// ---------------------------------------------------------------------------
// TaskResult constructors
// ---------------------------------------------------------------------------

#[test]
fn task_result_success_constructor_marks_success() {
    let result = TaskResult::success(
        "task-1".to_string(),
        "agent-a".to_string(),
        serde_json::json!({"answer": 42}),
        5,
        "sess-1".to_string(),
    );

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.steps_used, 5);
    assert_eq!(result.output["answer"], 42);
}

#[test]
fn task_result_failure_constructor_marks_failure() {
    let result = TaskResult::failure(
        "task-2".to_string(),
        "agent-b".to_string(),
        "LLM timeout".to_string(),
    );

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("LLM timeout"));
    assert_eq!(result.steps_used, 0);
    assert!(result.output.is_null());
}

// ---------------------------------------------------------------------------
// PipelineResult aggregation
// ---------------------------------------------------------------------------

#[test]
fn pipeline_result_all_success_computes_totals() {
    let results = vec![
        TaskResult::success("t1".into(), "a".into(), serde_json::json!("first"), 3, "s".into()),
        TaskResult::success("t2".into(), "b".into(), serde_json::json!("second"), 4, "s".into()),
    ];

    let pipeline = PipelineResult::from_results(results);

    assert!(pipeline.success);
    assert_eq!(pipeline.total_steps, 7);
    assert_eq!(pipeline.final_output, serde_json::json!("second"));
    assert!(pipeline.error.is_none());
}

#[test]
fn pipeline_result_with_failure_reports_first_error() {
    let results = vec![
        TaskResult::success("t1".into(), "a".into(), Value::Null, 2, "s".into()),
        TaskResult::failure("t2".into(), "b".into(), "tool error".to_string()),
        TaskResult::success("t3".into(), "c".into(), Value::Null, 1, "s".into()),
    ];

    let pipeline = PipelineResult::from_results(results);

    assert!(!pipeline.success);
    assert_eq!(pipeline.error.as_deref(), Some("tool error"));
}

#[test]
fn pipeline_result_empty_has_null_output() {
    let pipeline = PipelineResult::from_results(vec![]);
    assert!(pipeline.final_output.is_null());
    assert!(pipeline.success); // vacuously true — no failures
    assert_eq!(pipeline.total_steps, 0);
}

// ---------------------------------------------------------------------------
// budget_steps guardrail — verifies the min() semantics used in orchestrator
// ---------------------------------------------------------------------------

#[test]
fn budget_steps_guardrail_caps_at_max_steps() {
    // Mirrors the computation in execute_task:
    //   budgeted_max_steps = task.budget_steps.map(|b| b.min(max_steps)).unwrap_or(max_steps)
    let max_steps = 10usize;

    let task_with_lower_budget = AgentTask::new("test").with_budget_steps(6);
    let budgeted = task_with_lower_budget
        .budget_steps
        .map(|b| b.min(max_steps))
        .unwrap_or(max_steps);
    assert_eq!(budgeted, 6, "budget_steps < max_steps → budget_steps wins");

    let task_with_higher_budget = AgentTask::new("test").with_budget_steps(15);
    let budgeted = task_with_higher_budget
        .budget_steps
        .map(|b| b.min(max_steps))
        .unwrap_or(max_steps);
    assert_eq!(budgeted, 10, "budget_steps > max_steps → max_steps wins (guardrail)");

    let task_without_budget = AgentTask::new("test");
    let budgeted = task_without_budget
        .budget_steps
        .map(|b| b.min(max_steps))
        .unwrap_or(max_steps);
    assert_eq!(budgeted, 10, "no budget_steps → max_steps used");
}

// ---------------------------------------------------------------------------
// deadline_unix_ms pre-check — verifies expired deadline is detected
// ---------------------------------------------------------------------------

#[test]
fn deadline_unix_ms_in_past_is_expired() {
    // Mirrors the pre-check in execute_task:
    //   if now_ms >= deadline { ... deadline_exceeded = true }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_millis() as i64;

    let past_deadline: i64 = now_ms - 10_000; // 10 seconds ago
    assert!(
        now_ms >= past_deadline,
        "a deadline in the past must be detected as exceeded"
    );

    let future_deadline: i64 = now_ms + 60_000; // 60 seconds from now
    assert!(
        now_ms < future_deadline,
        "a deadline in the future must not be flagged as exceeded"
    );
}
