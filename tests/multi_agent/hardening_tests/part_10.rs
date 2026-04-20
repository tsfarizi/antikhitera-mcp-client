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

