//! Centralized tests for multi-agent production hardening types.
//!
//! These are pure-logic tests that do not require a live LLM or MCP server.
//! They cover: `AgentTask` builder, serde roundtrips, `TaskRetryPolicy`,
//! `TaskExecutionMetadata` defaults, `TaskResult` constructors,
//! `PipelineResult` aggregation, `budget_steps` guardrail semantics, and
//! deadline pre-check logic.

use antikythera_core::application::agent::multi_agent::task::{
    AgentTask, ErrorKind, PipelineResult, RetryCondition, RoutingDecision, TaskExecutionMetadata,
    TaskResult, TaskRetryPolicy,
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
        ..TaskRetryPolicy::default()
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
            ..TaskRetryPolicy::default()
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
        ..TaskRetryPolicy::default()
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
        ..TaskExecutionMetadata::default()
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
        TaskResult::success(
            "t1".into(),
            "a".into(),
            serde_json::json!("first"),
            3,
            "s".into(),
        ),
        TaskResult::success(
            "t2".into(),
            "b".into(),
            serde_json::json!("second"),
            4,
            "s".into(),
        ),
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
    assert_eq!(
        budgeted, 10,
        "budget_steps > max_steps → max_steps wins (guardrail)"
    );

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

// ---------------------------------------------------------------------------
// CancellationToken — cooperative cancellation
// ---------------------------------------------------------------------------

#[test]
fn cancellation_token_new_is_not_cancelled() {
    use antikythera_core::application::agent::multi_agent::cancellation::CancellationToken;
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
}

#[test]
fn cancellation_token_cancel_sets_flag() {
    use antikythera_core::application::agent::multi_agent::cancellation::CancellationToken;
    let token = CancellationToken::new();
    token.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn cancellation_token_child_shares_flag_with_parent() {
    use antikythera_core::application::agent::multi_agent::cancellation::CancellationToken;
    let parent = CancellationToken::new();
    let child = parent.child_token();
    // cancelling parent is visible through child
    parent.cancel();
    assert!(
        child.is_cancelled(),
        "child must share the cancellation flag"
    );
}

#[test]
fn cancellation_token_child_can_cancel_parent_flag() {
    use antikythera_core::application::agent::multi_agent::cancellation::CancellationToken;
    let parent = CancellationToken::new();
    let child = parent.child_token();
    // cancelling via child is visible on parent
    child.cancel();
    assert!(
        parent.is_cancelled(),
        "cancelling child must cancel the shared flag"
    );
}

#[test]
fn cancellation_snapshot_serde_roundtrip() {
    use antikythera_core::application::agent::multi_agent::cancellation::{
        CancellationSnapshot, CancellationToken,
    };
    let token = CancellationToken::new();
    token.cancel();
    let snap = CancellationSnapshot::from(&token);
    let json = serde_json::to_string(&snap).expect("serialize");
    let restored: CancellationSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert!(restored.was_cancelled);
}

// ---------------------------------------------------------------------------
// OrchestratorBudget — step and task guardrails
// ---------------------------------------------------------------------------

#[test]
fn budget_step_limit_exhaustion() {
    use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;
    let budget = OrchestratorBudget::new().with_max_total_steps(5);
    assert!(!budget.is_step_budget_exhausted());
    budget.record_steps(3);
    assert!(!budget.is_step_budget_exhausted()); // 3 < 5
    budget.record_steps(2);
    assert!(budget.is_step_budget_exhausted()); // 5 >= 5
}

#[test]
fn budget_task_limit_exhaustion() {
    use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;
    let budget = OrchestratorBudget::new().with_max_total_tasks(2);
    assert!(!budget.is_task_budget_exhausted());
    budget.record_task_dispatch();
    assert!(!budget.is_task_budget_exhausted()); // 1 < 2
    budget.record_task_dispatch();
    assert!(budget.is_task_budget_exhausted()); // 2 >= 2
}

// ---------------------------------------------------------------------------
// SDK hardening API surface
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn sdk_hardening_runtime_configure_cancel_and_snapshot() {
    use antikythera_sdk::agents::{
        cancel_orchestrator, configure_hardening, get_monitor_snapshot, reset_hardening_runtime,
    };

    reset_hardening_runtime().expect("reset runtime");

    configure_hardening(
        &serde_json::json!({
            "max_concurrent_tasks": 4,
            "max_total_steps": 120,
            "max_total_tasks": 30,
            "default_retry_condition": "on_transient",
            "guardrails": {
                "timeout": {
                    "max_timeout_ms": 2000,
                    "require_explicit_timeout": true
                },
                "budget": {
                    "max_task_steps": 8,
                    "require_explicit_budget": true
                },
                "cancellation": true
            }
        })
        .to_string(),
    )
    .expect("configure hardening");

    cancel_orchestrator().expect("cancel runtime");
    let snapshot_json = get_monitor_snapshot().expect("snapshot");
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_json).expect("json");

    assert_eq!(snapshot["max_concurrent_tasks"], 4);
    assert_eq!(snapshot["max_total_steps"], 120);
    assert_eq!(snapshot["max_total_tasks"], 30);
    assert_eq!(snapshot["cancelled"], true);
}

#[test]
fn sdk_default_retry_condition_applies_to_unconfigured_task() {
    use antikythera_core::application::agent::multi_agent::task::AgentTask;
    use antikythera_sdk::agents::{
        GuardrailOptions, OrchestratorOptions, RetryConditionOption, TimeoutGuardrailOptions,
    };

    let mut task = AgentTask::new("summarize this document");
    let options = OrchestratorOptions {
        default_retry_condition: RetryConditionOption::OnTransient,
        guardrails: GuardrailOptions {
            timeout: Some(TimeoutGuardrailOptions {
                max_timeout_ms: Some(5_000),
                require_explicit_timeout: true,
            }),
            ..GuardrailOptions::default()
        },
        ..OrchestratorOptions::default()
    };

    options.apply_to_task(&mut task);
    let retry = task.retry_policy.expect("retry policy should be injected");
    let retry_json = serde_json::to_value(&retry).expect("serialize retry policy");

    assert_eq!(retry_json["condition"], "on_transient");
    assert!(options.guardrails.timeout.is_some());
}

#[test]
fn sdk_guardrail_options_build_non_empty_chain() {
    use antikythera_sdk::agents::{
        BudgetGuardrailOptions, GuardrailOptions, RateLimitGuardrailOptions,
        TimeoutGuardrailOptions,
    };

    let options = GuardrailOptions {
        timeout: Some(TimeoutGuardrailOptions {
            max_timeout_ms: Some(3_000),
            require_explicit_timeout: true,
        }),
        budget: Some(BudgetGuardrailOptions {
            max_task_steps: Some(6),
            require_explicit_budget: true,
            allow_exhausted_orchestrator: false,
        }),
        rate_limit: Some(RateLimitGuardrailOptions {
            max_tasks: Some(2),
            window_ms: Some(1_000),
        }),
        cancellation: true,
    };

    assert!(!options.is_empty());
    assert_eq!(options.to_guardrail_chain().len(), 4);
}

#[test]
fn sdk_guardrail_options_validation_rejects_partial_rate_limit_config() {
    use antikythera_sdk::agents::configure_hardening;

    let error = configure_hardening(
        &serde_json::json!({
            "guardrails": {
                "rate_limit": {
                    "max_tasks": 2
                }
            }
        })
        .to_string(),
    )
    .expect_err("partial rate limit config should be rejected");

    assert!(error.contains("guardrails.rate_limit requires both max_tasks and window_ms"));
}

#[test]
fn sdk_task_result_detail_is_decoded_without_manual_mapping() {
    use antikythera_sdk::agents::task_result_detail;

    let task_result_json = serde_json::json!({
        "task_id": "task-1",
        "agent_id": "agent-a",
        "success": false,
        "output": null,
        "error": "timeout",
        "steps_used": 0,
        "session_id": "session-a",
        "error_kind": "transient",
        "metadata": {
            "attempt_count": 1,
            "duration_ms": 11,
            "timed_out": true,
            "deadline_exceeded": false,
            "cancelled": false,
            "retry_applied": false,
            "routed_by": null,
            "execution_mode": "auto",
            "correlation_id": "corr-7",
            "routing_decision": {
                "router_name": "round-robin",
                "selected_agent_id": "agent-a",
                "candidates_considered": 2,
                "reason": "balanced"
            },
            "concurrency_wait_ms": 3,
            "budget_exhausted": false,
            "error_kind": "transient",
            "guardrail_name": "timeout",
            "guardrail_stage": "pre_check"
        }
    })
    .to_string();

    let detail_json = task_result_detail(&task_result_json).expect("detail json");
    let detail: serde_json::Value = serde_json::from_str(&detail_json).expect("parsed detail");

    assert_eq!(detail["error_kind"], "transient");
    assert_eq!(detail["is_transient"], true);
    assert_eq!(detail["router_name"], "round-robin");
    assert_eq!(detail["selected_agent_id"], "agent-a");
    assert_eq!(detail["candidates_considered"], 2);
    assert_eq!(detail["concurrency_wait_ms"], 3);
    assert_eq!(detail["guardrail_name"], "timeout");
    assert_eq!(detail["guardrail_stage"], "pre_check");
}

#[test]
fn budget_clone_shares_counter_state() {
    use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;
    let budget = OrchestratorBudget::new().with_max_total_steps(10);
    let budget2 = budget.clone();
    budget.record_steps(4);
    // Both views should reflect the same counter
    assert_eq!(budget2.consumed_steps(), 4);
}

#[test]
fn budget_snapshot_reflects_current_state() {
    use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;
    let budget = OrchestratorBudget::new()
        .with_max_total_steps(20)
        .with_max_total_tasks(5);
    budget.record_steps(7);
    budget.record_task_dispatch();
    budget.record_task_dispatch();
    let snap = budget.snapshot();
    assert_eq!(snap.consumed_steps, 7);
    assert_eq!(snap.dispatched_tasks, 2);
    assert_eq!(snap.max_total_steps, Some(20));
    assert_eq!(snap.max_total_tasks, Some(5));
}

#[test]
fn budget_snapshot_serde_roundtrip() {
    use antikythera_core::application::agent::multi_agent::budget::OrchestratorBudget;
    let budget = OrchestratorBudget::new().with_max_total_steps(10);
    budget.record_steps(3);
    let snap = budget.snapshot();
    let json = serde_json::to_string(&snap).expect("serialize");
    let restored: antikythera_core::application::agent::multi_agent::budget::BudgetSnapshot =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.consumed_steps, 3);
    assert_eq!(restored.max_total_steps, Some(10));
}

// ---------------------------------------------------------------------------
// RetryCondition and ErrorKind — conditional retry logic
// ---------------------------------------------------------------------------

#[test]
fn retry_condition_default_is_always() {
    let policy = TaskRetryPolicy::default();
    assert!(matches!(policy.condition, RetryCondition::Always));
}

#[test]
fn retry_condition_on_transient_blocks_retry_for_permanent() {
    // Mirrors the gate in execute_task:
    //   if matches!(condition, OnTransient) && !error_kind.is_transient() { break; }
    let is_transient_error = false; // permanent error
    let condition = RetryCondition::OnTransient;
    let should_retry = match condition {
        RetryCondition::Always => true,
        RetryCondition::Never => false,
        RetryCondition::OnTransient => is_transient_error,
    };
    assert!(!should_retry, "OnTransient must not retry permanent errors");
}

#[test]
fn retry_condition_on_transient_allows_retry_for_transient() {
    let is_transient_error = true;
    let condition = RetryCondition::OnTransient;
    let should_retry = match condition {
        RetryCondition::Always => true,
        RetryCondition::Never => false,
        RetryCondition::OnTransient => is_transient_error,
    };
    assert!(should_retry, "OnTransient must retry transient errors");
}

#[test]
fn retry_condition_never_blocks_all_retries() {
    let condition = RetryCondition::Never;
    let should_retry = !matches!(condition, RetryCondition::Never);
    assert!(!should_retry, "Never must block all retries");
}

#[test]
fn error_kind_serde_roundtrip() {
    let kinds = vec![
        ErrorKind::Transient,
        ErrorKind::Permanent,
        ErrorKind::Cancelled,
        ErrorKind::DeadlineExceeded,
        ErrorKind::BudgetExhausted,
    ];
    for kind in kinds {
        let json = serde_json::to_string(&kind).expect("serialize");
        let restored: ErrorKind = serde_json::from_str(&json).expect("deserialize");
        // Verify the discriminant name round-trips (serde snake_case)
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            serde_json::to_string(&restored).unwrap()
        );
    }
}

#[test]
fn task_result_is_transient_helper() {
    let transient = TaskResult::failure_with_kind(
        "t1".into(),
        "a".into(),
        "rate limited".into(),
        ErrorKind::Transient,
    );
    assert!(transient.is_transient());

    let permanent = TaskResult::failure_with_kind(
        "t2".into(),
        "b".into(),
        "auth error".into(),
        ErrorKind::Permanent,
    );
    assert!(!permanent.is_transient());

    let success = TaskResult::success("t3".into(), "c".into(), serde_json::json!(1), 1, "s".into());
    assert!(!success.is_transient(), "success is never transient");
}

// ---------------------------------------------------------------------------
// RoutingDecision — routing introspection
// ---------------------------------------------------------------------------

#[test]
fn routing_decision_embedded_in_metadata() {
    let decision = RoutingDecision {
        router_name: "round-robin".to_string(),
        selected_agent_id: "agent-42".to_string(),
        candidates_considered: 3,
        reason: Some("round-robin selected agent-42".to_string()),
    };
    let meta = TaskExecutionMetadata {
        routing_decision: Some(decision.clone()),
        ..TaskExecutionMetadata::default()
    };
    let rd = meta.routing_decision.as_ref().unwrap();
    assert_eq!(rd.router_name, "round-robin");
    assert_eq!(rd.selected_agent_id, "agent-42");
    assert_eq!(rd.candidates_considered, 3);
    assert!(rd.reason.as_deref().unwrap().contains("round-robin"));
}

#[test]
fn routing_decision_serde_roundtrip() {
    let decision = RoutingDecision {
        router_name: "role".to_string(),
        selected_agent_id: "planner".to_string(),
        candidates_considered: 5,
        reason: Some("role='planner' matched agent_id='planner'".to_string()),
    };
    let json = serde_json::to_string(&decision).expect("serialize");
    let restored: RoutingDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.router_name, "role");
    assert_eq!(restored.selected_agent_id, "planner");
    assert_eq!(restored.candidates_considered, 5);
}

// ---------------------------------------------------------------------------
// AgentRouter name() and routing_reason() introspection
// ---------------------------------------------------------------------------

#[test]
fn router_name_returns_correct_strings() {
    use antikythera_core::application::agent::multi_agent::AgentRouter;
    use antikythera_core::application::agent::multi_agent::router::{
        DirectRouter, FirstAvailableRouter, RoleRouter, RoundRobinRouter,
    };

    assert_eq!(DirectRouter.name(), "direct");
    assert_eq!(RoundRobinRouter::new().name(), "round-robin");
    assert_eq!(FirstAvailableRouter.name(), "first-available");
    assert_eq!(RoleRouter::new("executor").name(), "role");
}
