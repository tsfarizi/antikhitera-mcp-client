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
