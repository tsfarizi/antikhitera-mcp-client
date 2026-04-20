//! Centralized tests for the WASM agent runner.
//!
//! Covers: session lifecycle, commit flows (plain text + structured tool call),
//! streaming commit, telemetry counters, global context-policy update,
//! and rolling summarization with the `KeepBalanced` truncation strategy.

use antikythera_sdk::wasm_agent::runner::{
    append_llm_chunk, commit_llm_response, commit_llm_stream, drain_events, get_state,
    get_telemetry_snapshot, get_tools_prompt, init, prepare_user_turn, register_tools,
    set_context_policy,
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
#[serial_test::serial]
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
#[serial_test::serial]
fn structured_tool_call_commit_returns_call_tool() {
    register_tools(
        &serde_json::json!([
            {
                "name": "weather.get",
                "description": "Get weather",
                "parameters": [
                    {"name": "city", "param_type": "string", "description": "City", "required": true}
                ]
            }
        ])
        .to_string(),
    )
    .unwrap();

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
#[serial_test::serial]
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
#[serial_test::serial]
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
// 5. set_context_policy -- global policy applied in next turn
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn set_context_policy_applies_global_policy_on_next_turn() {
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

    // Override global policy to summarize after only 4 messages.
    // Since history is already 8 messages, this triggers on next prepare.
    let override_ok = set_context_policy(
        &serde_json::json!({
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

    // Next prepare without inline context_policy uses global override policy.
    prepare_user_turn(
        &serde_json::json!({
            "prompt": "trigger summary",
            "session_id": session_id,
            "system_prompt": "assistant"
        })
        .to_string(),
    )
    .unwrap();

    let state_json = get_state(&session_id).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();
    assert!(
        !state["rolling_summary"].is_null(),
        "rolling_summary should have been created by the updated global context policy"
    );
}

// ---------------------------------------------------------------------------
// 6. KeepBalanced truncation -- head and tail of history are retained
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
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

// ---------------------------------------------------------------------------
// 7. Tool registry -- register_tools and get_tools_prompt
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn register_tools_counts_tools_correctly() {
    let tools_json = serde_json::json!([
        {
            "name": "weather.get",
            "description": "Get current weather for a city",
            "parameters": [
                {"name": "city", "param_type": "string", "description": "City name", "required": true}
            ]
        },
        {
            "name": "calculator.add",
            "description": "Add two numbers",
            "parameters": [
                {"name": "a", "param_type": "number", "description": "First operand", "required": true},
                {"name": "b", "param_type": "number", "description": "Second operand", "required": true}
            ]
        }
    ])
    .to_string();

    let count = register_tools(&tools_json).unwrap();
    assert_eq!(count, 2, "expected 2 tools registered");
}

#[test]
#[serial_test::serial]
fn get_tools_prompt_contains_tool_names() {
    let tools_json = serde_json::json!([
        {
            "name": "search.query",
            "description": "Search the web",
            "parameters": [
                {"name": "query", "param_type": "string", "description": "Search query", "required": true}
            ]
        }
    ])
    .to_string();

    register_tools(&tools_json).unwrap();
    let prompt = get_tools_prompt().unwrap();
    assert!(
        prompt.contains("search.query"),
        "tools prompt should contain tool name 'search.query'"
    );
    assert!(
        prompt.contains("query*"),
        "required param should be marked with '*'"
    );
}

// ---------------------------------------------------------------------------
// 8. Tool registry -- validation blocks unknown tool calls
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn unknown_tool_call_returns_error_when_registry_populated() {
    // Register only one known tool
    let tools_json = serde_json::json!([
        {
            "name": "weather.get",
            "description": "Get weather",
            "parameters": [
                {"name": "city", "param_type": "string", "description": "City", "required": true}
            ]
        }
    ])
    .to_string();
    register_tools(&tools_json).unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "book a flight")).unwrap();

    // LLM tries to call a tool not in the registry
    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "flights.book",
        "input": {"destination": "Tokyo"}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(result.is_err(), "should reject unknown tool call");
    let err = result.unwrap_err();
    assert!(
        err.contains("flights.book"),
        "error should mention the unknown tool name, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// 9. Tool registry -- validation blocks missing required param
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn missing_required_param_returns_error() {
    // Use a unique tool name to avoid cross-test registry pollution
    let tools_json = serde_json::json!([
        {
            "name": "geo.lookup",
            "description": "Lookup geo coordinates",
            "parameters": [
                {"name": "lat", "param_type": "number", "description": "Latitude", "required": true},
                {"name": "lon", "param_type": "number", "description": "Longitude", "required": true}
            ]
        }
    ])
    .to_string();
    register_tools(&tools_json).unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "find location")).unwrap();

    // LLM omits 'lon' (required)
    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "geo.lookup",
        "input": {"lat": 1.28}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(result.is_err(), "should reject call with missing required param");
    let err = result.unwrap_err();
    assert!(
        err.contains("lon"),
        "error should mention the missing param, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// 10. Tool registry -- valid call passes through when registry populated
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn valid_tool_call_passes_validation() {
    let tools_json = serde_json::json!([
        {
            "name": "db.query",
            "description": "Run a database query",
            "parameters": [
                {"name": "sql", "param_type": "string", "description": "SQL statement", "required": true}
            ]
        }
    ])
    .to_string();
    register_tools(&tools_json).unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "list users")).unwrap();

    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "db.query",
        "input": {"sql": "SELECT * FROM users"}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(result.is_ok(), "valid tool call should pass, got: {:?}", result);
    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(value["action"], "call_tool");
    assert_eq!(value["tool_name"], "db.query");
}

// ---------------------------------------------------------------------------
// 11. Tool registry -- empty registry allows any tool call (backward compat)
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn empty_registry_allows_any_tool_call() {
    // Clear the registry
    register_tools("[]").unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "do something")).unwrap();

    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "anything.goes",
        "input": {}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(result.is_ok(), "empty registry should allow any tool call");
}

