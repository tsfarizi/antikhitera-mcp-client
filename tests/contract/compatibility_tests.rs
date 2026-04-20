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

// Split into 5 parts for consistent test organization.
include!("compatibility_tests/part_01.rs");
include!("compatibility_tests/part_02.rs");
include!("compatibility_tests/part_03.rs");
include!("compatibility_tests/part_04.rs");
include!("compatibility_tests/part_05.rs");
