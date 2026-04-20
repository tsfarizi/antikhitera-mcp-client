use antikythera_sdk::wasm_agent::runner::{
    append_llm_chunk, commit_llm_response, commit_llm_stream, drain_events, hydrate_session, init,
    prepare_user_turn, process_tool_result_for_session, sweep_idle_sessions,
};

#[test]
#[serial_test::serial]
fn replay_trace_prepare_commit_tool_result_commit_final_is_deterministic() {
    let session_id = init(
        &serde_json::json!({
            "session_id": "replay-trace-session",
            "max_steps": 10,
            "session_timeout_secs": 3600,
            "max_in_memory_sessions": 16
        })
        .to_string(),
    )
    .unwrap();

    let prepared_1 = prepare_user_turn(
        &serde_json::json!({
            "prompt": "Cari cuaca",
            "session_id": session_id,
            "force_json": true,
            "correlation_id": "trace-1"
        })
        .to_string(),
    )
    .unwrap();

    let commit_1 = commit_llm_response(
        &prepared_1,
        &serde_json::json!({
            "action": "call_tool",
            "tool": "weather.get",
            "input": {"city": "Bandung"}
        })
        .to_string(),
    )
    .unwrap();
    let c1: serde_json::Value = serde_json::from_str(&commit_1).unwrap();
    assert_eq!(c1["action"], "call_tool");

    let tool_result = process_tool_result_for_session(
        "replay-trace-session",
        &serde_json::json!({
            "tool_name": "weather.get",
            "success": true,
            "output_json": "{\"temp\":24,\"condition\":\"cloudy\"}",
            "error_message": null,
            "correlation_id": "trace-1"
        })
        .to_string(),
    )
    .unwrap();
    let tr: serde_json::Value = serde_json::from_str(&tool_result).unwrap();
    assert_eq!(tr["tool_result"]["success"], true);

    let prepared_2 = prepare_user_turn(
        &serde_json::json!({
            "prompt": "Ringkas hasilnya",
            "session_id": "replay-trace-session",
            "correlation_id": "trace-1"
        })
        .to_string(),
    )
    .unwrap();

    let commit_2 = commit_llm_response(&prepared_2, "Cuaca Bandung 24C dan berawan.").unwrap();
    let c2: serde_json::Value = serde_json::from_str(&commit_2).unwrap();
    assert_eq!(c2["action"], "final");
}

#[test]
#[serial_test::serial]
fn malformed_tool_result_json_is_rejected() {
    let session_id = init(
        &serde_json::json!({
            "session_id": "malformed-tool-session",
            "max_steps": 10
        })
        .to_string(),
    )
    .unwrap();

    let _prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "hi",
            "session_id": session_id
        })
        .to_string(),
    )
    .unwrap();

    let err = process_tool_result_for_session(
        "malformed-tool-session",
        &serde_json::json!({
            "tool_name": "bad.tool",
            "success": true,
            "output_json": "{invalid-json",
            "error_message": null,
            "correlation_id": "trace-malformed"
        })
        .to_string(),
    )
    .unwrap_err();

    assert!(err.contains("Invalid tool output_json"));
}

#[test]
#[serial_test::serial]
fn partial_output_stream_is_committed_deterministically() {
    let session_id = init(
        &serde_json::json!({
            "session_id": "partial-stream-session",
            "max_steps": 10
        })
        .to_string(),
    )
    .unwrap();

    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "stream this",
            "session_id": session_id,
            "correlation_id": "trace-stream"
        })
        .to_string(),
    )
    .unwrap();

    append_llm_chunk(
        "partial-stream-session",
        "{\"response\":\"par",
        Some("trace-stream"),
    )
    .unwrap();
    append_llm_chunk("partial-stream-session", "tial\"}", Some("trace-stream")).unwrap();

    let committed = commit_llm_stream(&prepared).unwrap();
    let value: serde_json::Value = serde_json::from_str(&committed).unwrap();
    assert_eq!(value["action"], "final");

    let events: serde_json::Value =
        serde_json::from_str(&drain_events("partial-stream-session").unwrap()).unwrap();
    assert!(
        events
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "llm_chunk"),
        "llm_chunk events should be present for partial stream"
    );
}

#[test]
#[serial_test::serial]
fn timeout_mode_is_deterministic_via_sweep_and_hydrate() {
    let session_id = init(
        &serde_json::json!({
            "session_id": "timeout-replay-session",
            "max_steps": 10,
            "session_timeout_secs": 1,
            "max_in_memory_sessions": 8
        })
        .to_string(),
    )
    .unwrap();

    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "hello",
            "session_id": session_id,
            "correlation_id": "trace-timeout"
        })
        .to_string(),
    )
    .unwrap();
    let _ = commit_llm_response(&prepared, "ok").unwrap();

    let archived =
        sweep_idle_sessions(Some(chrono::Utc::now().timestamp_millis() + 2_000)).unwrap();
    assert_eq!(archived, 1);

    let events: serde_json::Value =
        serde_json::from_str(&drain_events("timeout-replay-session").unwrap()).unwrap();
    let state_json = events
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["kind"] == "session_archived")
        .and_then(|event| event["payload"]["state_json"].as_str())
        .unwrap()
        .to_string();

    hydrate_session("timeout-replay-session", &state_json).unwrap();
    let prepared_again = prepare_user_turn(
        &serde_json::json!({
            "prompt": "after timeout",
            "session_id": "timeout-replay-session",
            "correlation_id": "trace-timeout"
        })
        .to_string(),
    )
    .unwrap();
    let final_commit = commit_llm_response(&prepared_again, "restored and running").unwrap();
    let final_v: serde_json::Value = serde_json::from_str(&final_commit).unwrap();
    assert_eq!(final_v["action"], "final");
}
