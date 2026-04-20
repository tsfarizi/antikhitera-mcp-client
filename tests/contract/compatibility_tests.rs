use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use antikythera_sdk::wasm_agent::runner::{
    commit_llm_response, get_slo_snapshot, init, prepare_user_turn,
    process_tool_result_for_session, reset_session,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("contracts")
        .join(name)
}

fn normalize_decl(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_wit_signatures(wit_content: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut current_interface: Option<&str> = None;
    let mut in_decl = false;
    let mut decl_buffer = String::new();

    for line in wit_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("interface host-imports") {
            current_interface = Some("host-imports");
            continue;
        }
        if trimmed.starts_with("interface agent-runner") {
            current_interface = Some("agent-runner");
            continue;
        }
        if trimmed == "}" {
            current_interface = None;
            in_decl = false;
            decl_buffer.clear();
            continue;
        }

        if current_interface.is_none() || trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }

        if !in_decl && trimmed.contains('(') {
            in_decl = true;
            decl_buffer.clear();
            decl_buffer.push_str(trimmed);
            if trimmed.ends_with(';') {
                in_decl = false;
                if decl_buffer.contains("-> result<") {
                    output.push(format!(
                        "{}::{}",
                        current_interface.unwrap_or_default(),
                        normalize_decl(&decl_buffer)
                    ));
                }
            }
            continue;
        }

        if in_decl {
            decl_buffer.push(' ');
            decl_buffer.push_str(trimmed);
            if trimmed.ends_with(';') {
                in_decl = false;
                if decl_buffer.contains("-> result<") {
                    output.push(format!(
                        "{}::{}",
                        current_interface.unwrap_or_default(),
                        normalize_decl(&decl_buffer)
                    ));
                }
            }
        }
    }

    output
}

fn sorted_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .map(|m| m.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    keys
}

#[test]
#[serial_test::serial]
fn wit_contract_signatures_match_golden() {
    let wit_file = repo_root().join("wit").join("antikythera.wit");
    let wit_content = fs::read_to_string(wit_file).expect("read WIT file");

    let actual = extract_wit_signatures(&wit_content);
    let expected_path = fixture_path("wit_signatures.golden.txt");
    let expected = fs::read_to_string(expected_path)
        .expect("read golden WIT signatures")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        actual, expected,
        "WIT contract changed; this is a breaking-contract detector failure"
    );
}

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
