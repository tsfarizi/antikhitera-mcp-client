// ---------------------------------------------------------------------------
// 14. Hydrate session restores archived state and allows turn continuation
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn hydrate_session_restores_archived_state() {
    register_tools("[]").unwrap();

    let cfg = serde_json::json!({
        "max_steps": 10,
        "session_timeout_secs": 3600,
        "max_in_memory_sessions": 1
    })
    .to_string();

    let archived_id = init(&cfg).unwrap();
    let first = prepare_user_turn(&prepare_request(&archived_id, "message-1")).unwrap();
    commit_llm_response(&first, "reply-1").unwrap();

    let _other = init(&cfg).unwrap();
    let events: serde_json::Value =
        serde_json::from_str(&drain_events(&archived_id).unwrap()).unwrap();
    let state_json = events
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["kind"] == "session_archived")
        .and_then(|event| event["payload"]["state_json"].as_str())
        .expect("archived snapshot state_json should be present")
        .to_string();

    hydrate_session(&archived_id, &state_json).unwrap();

    let second = prepare_user_turn(&prepare_request(&archived_id, "message-2")).unwrap();
    let result = commit_llm_response(&second, "reply-2").unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["action"], "final");

    let restored_events: serde_json::Value =
        serde_json::from_str(&drain_events(&archived_id).unwrap()).unwrap();
    assert!(
        restored_events
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "session_restored"),
        "session_restored event should be emitted after hydration"
    );
}

