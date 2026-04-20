#[test]
#[serial_test::serial]
fn correlation_and_slo_contract_are_present() {
    let _ = reset_session("corr-slo-session");

    let session_id = init(
        &serde_json::json!({
            "session_id": "corr-slo-session",
            "max_steps": 10,
            "session_timeout_secs": 3600,
            "max_in_memory_sessions": 8
        })
        .to_string(),
    )
    .unwrap();

    let prepared = prepare_user_turn(
        &serde_json::json!({
            "prompt": "corr+slo",
            "session_id": session_id,
            "force_json": true,
            "correlation_id": "corr-e2e"
        })
        .to_string(),
    )
    .unwrap();

    let commit = commit_llm_response(&prepared, r#"{"action":"retry","error":"timeout"}"#).unwrap();
    let commit_v: serde_json::Value = serde_json::from_str(&commit).unwrap();
    assert_eq!(commit_v["action"], "retry");

    process_tool_result_for_session(
        "corr-slo-session",
        &serde_json::json!({
            "tool_name": "network.fetch",
            "success": false,
            "output_json": "{}",
            "error_message": "timeout",
            "correlation_id": "corr-e2e"
        })
        .to_string(),
    )
    .unwrap();

    let slo_v: serde_json::Value =
        serde_json::from_str(&get_slo_snapshot("corr-slo-session").unwrap()).unwrap();
    let keys = sorted_keys(&slo_v).into_iter().collect::<BTreeSet<_>>();

    for required in [
        "session_id",
        "correlation_id",
        "success_rate",
        "tool_error_rate",
        "retry_ratio",
        "p95_prepare_latency_ms",
        "p95_commit_latency_ms",
    ] {
        assert!(
            keys.contains(required),
            "missing required SLO key: {required}"
        );
    }

    assert_eq!(slo_v["correlation_id"], "corr-e2e");
}
