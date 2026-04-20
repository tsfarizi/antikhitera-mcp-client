use antikythera_sdk::{
    SloSnapshot, StreamEvent, TelemetrySnapshot, append_llm_chunk, commit_llm_response,
    commit_llm_stream, drain_events, get_agent_state, get_slo_snapshot, get_telemetry_snapshot,
    init_agent_runner, prepare_user_turn, reset_agent_session,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use antikythera_sdk::StreamEventKind;

use crate::error::{CliError, CliResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmStreamProbeReport {
    pub session_id: String,
    pub ffi_calls: Vec<String>,
    pub prepared_turn: serde_json::Value,
    pub commit_result: serde_json::Value,
    pub events: Vec<StreamEvent>,
    pub telemetry: TelemetrySnapshot,
    pub slo: SloSnapshot,
    pub state: serde_json::Value,
}

/// Execute a deterministic FFI + stream probe using the SDK runner surface.
///
/// This is a developer-facing diagnostic: it drives the same exported runtime
/// functions hosts use (`init`, `prepare_user_turn`, `append_llm_chunk`,
/// `commit_llm_stream`, `drain_events`, telemetry/slo snapshots).
pub fn run_wasm_stream_probe(
    prompt: &str,
    llm_payload: &str,
    stream_enabled: bool,
) -> CliResult<WasmStreamProbeReport> {
    let mut ffi_calls = Vec::new();

    let config_json = serde_json::json!({
        "max_steps": 12,
        "verbose": true,
        "auto_execute_tools": true,
        "session_timeout_secs": 300,
    })
    .to_string();

    ffi_calls.push("init".to_string());
    let session_id = init_agent_runner(&config_json).map_err(map_ffi_err("init"))?;

    let correlation_id = "cli-wasm-dev-stream";
    let prepare_json = serde_json::json!({
        "prompt": prompt,
        "session_id": session_id,
        "system_prompt": "You are a deterministic WASM harness probe.",
        "force_json": false,
        "correlation_id": correlation_id,
    })
    .to_string();

    ffi_calls.push("prepare_user_turn".to_string());
    let prepared_json =
        prepare_user_turn(&prepare_json).map_err(map_ffi_err("prepare_user_turn"))?;
    let prepared_turn: serde_json::Value =
        serde_json::from_str(&prepared_json).map_err(CliError::Serialization)?;

    let commit_result_json = if stream_enabled {
        let chunks = split_into_chunks(llm_payload, 3);
        for chunk in &chunks {
            ffi_calls.push("append_llm_chunk".to_string());
            append_llm_chunk(&session_id, chunk, Some(correlation_id))
                .map_err(map_ffi_err("append_llm_chunk"))?;
        }

        ffi_calls.push("commit_llm_stream".to_string());
        commit_llm_stream(&prepared_json).map_err(map_ffi_err("commit_llm_stream"))?
    } else {
        ffi_calls.push("commit_llm_response".to_string());
        commit_llm_response(&prepared_json, llm_payload)
            .map_err(map_ffi_err("commit_llm_response"))?
    };

    let commit_result: serde_json::Value =
        serde_json::from_str(&commit_result_json).map_err(CliError::Serialization)?;

    ffi_calls.push("drain_events".to_string());
    let events_json = drain_events(&session_id).map_err(map_ffi_err("drain_events"))?;
    let events: Vec<StreamEvent> =
        serde_json::from_str(&events_json).map_err(CliError::Serialization)?;

    ffi_calls.push("get_telemetry_snapshot".to_string());
    let telemetry_json =
        get_telemetry_snapshot(&session_id).map_err(map_ffi_err("get_telemetry_snapshot"))?;
    let telemetry: TelemetrySnapshot =
        serde_json::from_str(&telemetry_json).map_err(CliError::Serialization)?;

    ffi_calls.push("get_slo_snapshot".to_string());
    let slo_json = get_slo_snapshot(&session_id).map_err(map_ffi_err("get_slo_snapshot"))?;
    let slo: SloSnapshot = serde_json::from_str(&slo_json).map_err(CliError::Serialization)?;

    ffi_calls.push("get_state".to_string());
    let state_json = get_agent_state(&session_id).map_err(map_ffi_err("get_state"))?;
    let state: serde_json::Value =
        serde_json::from_str(&state_json).map_err(CliError::Serialization)?;

    ffi_calls.push("reset_session".to_string());
    let _ = reset_agent_session(&session_id).map_err(map_ffi_err("reset_session"))?;

    Ok(WasmStreamProbeReport {
        session_id,
        ffi_calls,
        prepared_turn,
        commit_result,
        events,
        telemetry,
        slo,
        state,
    })
}

pub fn render_wasm_stream_report(report: &WasmStreamProbeReport) -> CliResult<String> {
    let mut out = String::new();
    out.push_str("== WASM Dev Stream Probe ==\n");
    out.push_str(&format!("session_id: {}\n", report.session_id));
    out.push_str("ffi_calls: ");
    out.push_str(&report.ffi_calls.join(" -> "));
    out.push('\n');

    out.push_str("\n-- Commit Result --\n");
    out.push_str(
        &serde_json::to_string_pretty(&report.commit_result).map_err(CliError::Serialization)?,
    );
    out.push('\n');

    out.push_str("\n-- Stream Events --\n");
    for event in &report.events {
        let payload = serde_json::to_string(&event.payload).map_err(CliError::Serialization)?;
        out.push_str(&format!(
            "#{} step={} kind={:?} corr={:?} payload={}\n",
            event.seq, event.step, event.kind, event.correlation_id, payload
        ));
    }

    out.push_str("\n-- Telemetry Snapshot --\n");
    out.push_str(
        &serde_json::to_string_pretty(&report.telemetry).map_err(CliError::Serialization)?,
    );
    out.push('\n');

    out.push_str("\n-- SLO Snapshot --\n");
    out.push_str(&serde_json::to_string_pretty(&report.slo).map_err(CliError::Serialization)?);
    out.push('\n');

    Ok(out)
}

fn map_ffi_err(stage: &'static str) -> impl FnOnce(String) -> CliError {
    move |err| CliError::Validation(format!("WASM FFI stage '{stage}' failed: {err}"))
}

fn split_into_chunks(value: &str, max_chunks: usize) -> Vec<String> {
    if value.is_empty() || max_chunks <= 1 {
        return vec![value.to_string()];
    }

    let chunk_count = max_chunks.min(value.chars().count().max(1));
    let chars: Vec<char> = value.chars().collect();
    let chunk_size = chars.len().div_ceil(chunk_count);

    let mut chunks = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let end = (i + chunk_size).min(chars.len());
        chunks.push(chars[i..end].iter().collect());
        i = end;
    }

    if chunks.is_empty() {
        chunks.push(value.to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn wasm_probe_stream_captures_ffi_trace_and_events() {
        let report = run_wasm_stream_probe("ffi smoke", "{\"response\":\"ok\"}", true)
            .expect("probe should succeed");

        assert!(!report.session_id.is_empty());
        assert!(report.ffi_calls.iter().any(|c| c == "init"));
        assert!(report.ffi_calls.iter().any(|c| c == "prepare_user_turn"));
        assert!(report.ffi_calls.iter().any(|c| c == "append_llm_chunk"));
        assert!(report.ffi_calls.iter().any(|c| c == "commit_llm_stream"));
        assert!(report.ffi_calls.iter().any(|c| c == "drain_events"));

        assert!(
            report
                .events
                .iter()
                .any(|e| matches!(e.kind, StreamEventKind::UserTurnPrepared)),
            "expected UserTurnPrepared in stream events"
        );
        assert!(
            report
                .events
                .iter()
                .any(|e| matches!(e.kind, StreamEventKind::LlmChunk)),
            "expected LlmChunk in stream events"
        );
        assert!(
            report
                .events
                .iter()
                .any(|e| matches!(e.kind, StreamEventKind::LlmCommitted)),
            "expected LlmCommitted in stream events"
        );

        assert_eq!(
            report
                .commit_result
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "final"
        );

        assert!(report.telemetry.counters.turns_prepared >= 1);
        assert!(report.telemetry.counters.llm_commits >= 1);
    }

    #[test]
    #[serial_test::serial]
    fn rendered_report_contains_dev_sections() {
        let report = run_wasm_stream_probe("ffi report", "{\"response\":\"report\"}", true)
            .expect("probe should succeed");
        let rendered = render_wasm_stream_report(&report).expect("render should succeed");

        assert!(rendered.contains("== WASM Dev Stream Probe =="));
        assert!(rendered.contains("-- Stream Events --"));
        assert!(rendered.contains("-- Telemetry Snapshot --"));
        assert!(rendered.contains("-- SLO Snapshot --"));
    }
}
