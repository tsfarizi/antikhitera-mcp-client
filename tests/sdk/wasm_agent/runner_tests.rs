//! Centralized tests for the WASM agent runner.
//!
//! Covers: session lifecycle, commit flows (plain text + structured tool call),
//! streaming commit, telemetry counters, global context-policy update,
//! and rolling summarization with the `KeepBalanced` truncation strategy.

use antikythera_sdk::wasm_agent::runner::{
    append_llm_chunk, commit_llm_response, commit_llm_stream, drain_events, get_state,
    get_telemetry_snapshot, get_tools_prompt, hydrate_session, init, prepare_user_turn,
    register_tools, report_session_restore_progress, set_context_policy, sweep_idle_sessions,
};

// Split by concern to keep file size manageable and improve readability.
include!("runner_tests/part_01.rs");
include!("runner_tests/part_02.rs");
include!("runner_tests/part_03.rs");
include!("runner_tests/part_04.rs");
include!("runner_tests/part_05.rs");
include!("runner_tests/part_06.rs");
include!("runner_tests/part_07.rs");
include!("runner_tests/part_08.rs");
include!("runner_tests/part_09.rs");
include!("runner_tests/part_10.rs");
include!("runner_tests/part_11.rs");
include!("runner_tests/part_12.rs");
include!("runner_tests/part_13.rs");
include!("runner_tests/part_14.rs");
include!("runner_tests/part_15.rs");
include!("runner_tests/part_16.rs");
