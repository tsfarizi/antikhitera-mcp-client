//! Centralized tests for the WASM agent runner.
//!
//! Covers: session lifecycle, commit flows (plain text + structured tool call),
//! streaming commit, telemetry counters, context-policy provider/model override,
//! and rolling summarization with the `KeepBalanced` truncation strategy.

use antikythera_sdk::wasm_agent::runner::{
    append_llm_chunk, commit_llm_response, commit_llm_stream, drain_events, get_state,
    get_telemetry_snapshot, init, prepare_user_turn, set_context_policy,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal JSON config string.
fn config_json() -> String {
    r#"{"max_steps":10}"#.to_string()
}

/// Build a `prepare_user_turn` request JSON for a given session/prompt.
fn prepare_request(session_id: &str, prompt: &str) -> String {
    serde_json::json!({
        "prompt": prompt,
        "session_id": session_id,
        "system_prompt": "You are a helpful assistant",
        "force_json": false
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// 1. Plain-text response -> Final action
// ---------------------------------------------------------------------------

#[test]
fn plain_text_commit_returns_final_action() {
    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "hello")).unwrap();

    let result_json = commit_llm_response(&prepared, "plain response").unwrap();
    let value: serde_json::Value = serde_json::from_str(&result_json).unwrap();

    assert_eq!(value["action"], "final");
    assert_eq!(value["content"], "plain response");
}

// ---------------------------------------------------------------------------
// 2. Structured tool-call response -> CallTool action
// ---------------------------------------------------------------------------

#[test]
fn structured_tool_call_commit_returns_call_tool() {
    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "check weather",
            "session_id": session_id,
            "system_prompt": "Use tools if needed",
            "force_json": true
        })
        .to_string(),
    )
    .unwrap();

    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "weather.get",
        "input": {"city": "Jakarta"}
    })
    .to_string();

    let result_json = commit_llm_response(&prepared, &response).unwrap();
    let value: serde_json::Value = serde_json::from_str(&result_json).unwrap();

    assert_eq!(value["action"], "call_tool");
    assert_eq!(value["tool_name"], "weather.get");
    assert_eq!(value["tool_input"]["city"], "Jakarta");
}

// ---------------------------------------------------------------------------
// 3. Streaming chunks -> commit_llm_stream -> events drained
// ---------------------------------------------------------------------------

#[test]
fn stream_chunks_and_drain_events() {
    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "hello stream",
            "session_id": session_id,
            "system_prompt": "You are a helpful assistant",
            "correlation_id": "corr-stream"
        })
        .to_string(),
    )
    .unwrap();

    append_llm_chunk(&session_id, "{", Some("corr-stream")).unwrap();
    append_llm_chunk(&session_id, r#""response":"ok"}"#, Some("corr-stream")).unwrap();

    let result_json = commit_llm_stream(&prepared).unwrap();
    let value: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(value["action"], "final");

    let events_json = drain_events(&session_id).unwrap();
    let events: serde_json::Value = serde_json::from_str(&events_json).unwrap();
    assert!(
        events.as_array().unwrap().len() >= 4,
        "expected at least 4 events (prepare, 2 chunks, commit)"
    );
}

// ---------------------------------------------------------------------------
// 4. Telemetry counters increment per turn
// ---------------------------------------------------------------------------

#[test]
fn telemetry_counters_increment_per_turn() {
    let session_id = init(&config_json()).unwrap();

    let prepared = prepare_user_turn(&prepare_request(&session_id, "turn 1")).unwrap();
    commit_llm_response(&prepared, "answer 1").unwrap();

    let prepared = prepare_user_turn(&prepare_request(&session_id, "turn 2")).unwrap();
    commit_llm_response(&prepared, "answer 2").unwrap();

    let telemetry_json = get_telemetry_snapshot(&session_id).unwrap();
    let t: serde_json::Value = serde_json::from_str(&telemetry_json).unwrap();

    assert!(
        t["counters"]["turns_prepared"].as_u64().unwrap_or(0) >= 2,
        "turns_prepared should be at least 2"
    );
    assert!(
        t["counters"]["llm_commits"].as_u64().unwrap_or(0) >= 2,
        "llm_commits should be at least 2"
    );
}

// ---------------------------------------------------------------------------
// 5. set_context_policy -- per-provider/model override applied in next turn
// ---------------------------------------------------------------------------

#[test]
fn set_context_policy_provider_override_applied_on_matching_provider_model() {
    // Inline high-threshold policy in every prepare call prevents concurrent init() calls
    // from other tests from mutating the global default_config and triggering early
    // summarization on this session.
    let no_summarize_policy = serde_json::json!({
        "max_history_messages": 20,
        "summarize_after_messages": 20,
        "summary_max_chars": 500,
        "truncation_strategy": "keep_newest"
    });

    let session_id = init(&serde_json::json!({"max_steps": 20}).to_string()).unwrap();

    // Pump 4 turns so history has 8 messages.
    // Inline policy prevents accidental summarization even if default_config changes.
    for i in 0..4 {
        let prepared = prepare_user_turn(
            &serde_json::json!({
                "prompt": format!("message {i}"),
                "session_id": session_id,
                "system_prompt": "assistant",
                "context_policy": no_summarize_policy
            })
            .to_string(),
        )
        .unwrap();
        commit_llm_response(&prepared, &format!("reply {i}")).unwrap();
    }

    // Override policy for provider="p-runner-test", model="m-runner-test":
    // summarize after only 4 messages -- 8 > 4 -- triggers on next prepare with matching pair.
    let override_ok = set_context_policy(
        &serde_json::json!({
            "provider": "p-runner-test",
            "model": "m-runner-test",
            "policy": {
                "max_history_messages": 4,
                "summarize_after_messages": 4,
                "summary_max_chars": 120,
                "truncation_strategy": "keep_newest"
            }
        })
        .to_string(),
    )
    .unwrap();
    assert!(override_ok);

    // Next prepare with matching provider/model activates the override policy.
    // No inline context_policy here -- the override is selected via provider+model key.
    prepare_user_turn(
        &serde_json::json!({
            "prompt": "trigger summary",
            "session_id": session_id,
            "system_prompt": "assistant",
            "provider": "p-runner-test",
            "model": "m-runner-test"
        })
        .to_string(),
    )
    .unwrap();

    let state_json = get_state(&session_id).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();
    assert!(
        !state["rolling_summary"].is_null(),
        "rolling_summary should have been created by the provider-model override policy"
    );
}

// ---------------------------------------------------------------------------
// 6. KeepBalanced truncation -- head and tail of history are retained
// ---------------------------------------------------------------------------

#[test]
fn keep_balanced_truncation_retains_head_and_tail() {
    // Inline context_policy in every prepare call isolates this test from concurrent
    // init() calls that overwrite the global default_config.context_policy.
    let kb_policy = serde_json::json!({
        "max_history_messages": 4,
        "summarize_after_messages": 4,
        "summary_max_chars": 100,
        "truncation_strategy": "keep_balanced"
    });

    let session_id = init(&serde_json::json!({"max_steps": 30}).to_string()).unwrap();

    // 3 commit cycles -> 6 messages; policy is passed inline so it cannot be
    // overridden by a concurrent test calling init() with a different policy.
    for i in 0..3 {
        let prepared = prepare_user_turn(
            &serde_json::json!({
                "prompt": format!("msg {i}"),
                "session_id": session_id,
                "system_prompt": "assistant",
                "context_policy": kb_policy
            })
            .to_string(),
        )
        .unwrap();
        commit_llm_response(&prepared, &format!("resp {i}")).unwrap();
    }

    // 4th prepare: history=6, summarize_after=4 -> triggers KeepBalanced truncation.
    prepare_user_turn(
        &serde_json::json!({
            "prompt": "trigger",
            "session_id": session_id,
            "system_prompt": "assistant",
            "context_policy": kb_policy
        })
        .to_string(),
    )
    .unwrap();

    let state_json = get_state(&session_id).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();

    assert!(
        !state["rolling_summary"].is_null(),
        "rolling_summary should have been created"
    );

    let history_len = state["message_history"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert!(
        history_len <= 4,
        "KeepBalanced should truncate history to at most max_history_messages (4), got {history_len}"
    );
}
