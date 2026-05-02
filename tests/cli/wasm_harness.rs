use antikythera_cli::domain::use_cases::{render_wasm_stream_report, run_wasm_stream_probe};
use antikythera_sdk::StreamEventKind;
use serial_test::serial;

#[test]
#[serial]
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
    assert_eq!(
        report
            .capability_probes
            .get("json_schema_validation")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "ok"
    );
    assert_eq!(
        report
            .capability_probes
            .get("tool_call_validation")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "ok"
    );
}

#[test]
#[serial]
fn rendered_report_contains_dev_sections() {
    let report = run_wasm_stream_probe("ffi report", "{\"response\":\"report\"}", true)
        .expect("probe should succeed");
    let rendered = render_wasm_stream_report(&report).expect("render should succeed");

    assert!(rendered.contains("== WASM Dev Stream Probe =="));
    assert!(rendered.contains("-- Stream Events --"));
    assert!(rendered.contains("-- Telemetry Snapshot --"));
    assert!(rendered.contains("-- SLO Snapshot --"));
    assert!(rendered.contains("-- Capability Probes --"));
}
