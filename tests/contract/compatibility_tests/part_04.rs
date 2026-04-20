#[test]
#[serial_test::serial]
fn payload_contract_shapes_match_golden() {
    let _ = reset_session("contract-snap-session");

    let session_id = init(
        &serde_json::json!({
            "session_id": "contract-snap-session",
            "max_steps": 10,
            "session_timeout_secs": 3600,
            "max_in_memory_sessions": 8
        })
        .to_string(),
    )
    .unwrap();

    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "contract snapshot",
            "session_id": session_id,
            "system_prompt": "snapshot prompt",
            "force_json": true,
            "correlation_id": "corr-contract"
        })
        .to_string(),
    )
    .unwrap();

    let commit = commit_llm_response(
        &prepared,
        &serde_json::json!({
            "action": "call_tool",
            "tool": "weather.get",
            "input": {"city": "Jakarta"}
        })
        .to_string(),
    )
    .unwrap();

    let tool_processed = process_tool_result_for_session(
        "contract-snap-session",
        &serde_json::json!({
            "tool_name": "weather.get",
            "success": true,
            "output_json": "{\"temp\":30}",
            "error_message": null,
            "correlation_id": "corr-contract"
        })
        .to_string(),
    )
    .unwrap();

    let prepared_v: serde_json::Value = serde_json::from_str(&prepared).unwrap();
    let commit_v: serde_json::Value = serde_json::from_str(&commit).unwrap();
    let tool_v: serde_json::Value = serde_json::from_str(&tool_processed).unwrap();

    let golden_path = fixture_path("payload_contract.golden.json");
    let golden: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(golden_path).unwrap()).unwrap();

    assert_eq!(
        sorted_keys(&prepared_v),
        golden["prepared_turn_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        sorted_keys(&commit_v),
        golden["commit_result_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        sorted_keys(&tool_v),
        golden["tool_result_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        sorted_keys(&tool_v["tool_result"]),
        golden["tool_result_inner_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
}

