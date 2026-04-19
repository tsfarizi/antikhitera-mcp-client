//! Centralized unit tests for the WASM agent processor.
//!
//! Validates the generic JSON format contract (the only format WASM now accepts)
//! and the plain-text fallback.  Provider-native formats (OpenAI, Gemini,
//! Anthropic) are intentionally **not** tested here — that parsing is the
//! host's responsibility via FFI.

use antikythera_sdk::{AgentAction, AgentState, WasmAgentConfig, process_llm_response};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fresh_state() -> AgentState {
    AgentState::new(WasmAgentConfig::default())
}

// ---------------------------------------------------------------------------
// 1. Generic call_tool format
// ---------------------------------------------------------------------------

#[test]
fn generic_call_tool_action_is_parsed() {
    let mut state = fresh_state();
    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "calculator.add",
        "input": {"a": 1, "b": 2}
    })
    .to_string();

    let action = process_llm_response(&mut state, &response).unwrap();
    match action {
        AgentAction::CallTool { tool, input } => {
            assert_eq!(tool, "calculator.add");
            assert_eq!(input["a"], 1);
            assert_eq!(input["b"], 2);
        }
        other => panic!("expected CallTool, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. Generic final action via "action" field
// ---------------------------------------------------------------------------

#[test]
fn generic_final_action_in_action_field_is_parsed() {
    let mut state = fresh_state();
    let response = serde_json::json!({
        "action": "final",
        "response": "Task complete"
    })
    .to_string();

    let action = process_llm_response(&mut state, &response).unwrap();
    match action {
        AgentAction::Final { response } => {
            assert_eq!(response.as_str().unwrap(), "Task complete");
        }
        other => panic!("expected Final, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Shorthand "response" field → Final
// ---------------------------------------------------------------------------

#[test]
fn shorthand_response_field_is_treated_as_final() {
    let mut state = fresh_state();
    let response = serde_json::json!({ "response": "shorthand answer" }).to_string();

    let action = process_llm_response(&mut state, &response).unwrap();
    match action {
        AgentAction::Final { response } => {
            assert_eq!(response.as_str().unwrap(), "shorthand answer");
        }
        other => panic!("expected Final, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Shorthand "content" string field → Final
// ---------------------------------------------------------------------------

#[test]
fn shorthand_content_string_field_is_treated_as_final() {
    let mut state = fresh_state();
    let response = serde_json::json!({ "content": "content shorthand" }).to_string();

    let action = process_llm_response(&mut state, &response).unwrap();
    match action {
        AgentAction::Final { response } => {
            assert_eq!(response.as_str().unwrap(), "content shorthand");
        }
        other => panic!("expected Final, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Plain text (non-JSON) → Final with string value
// ---------------------------------------------------------------------------

#[test]
fn plain_text_non_json_is_treated_as_final() {
    let mut state = fresh_state();
    let action = process_llm_response(&mut state, "This is plain text").unwrap();
    match action {
        AgentAction::Final { response } => {
            assert_eq!(response.as_str().unwrap(), "This is plain text");
        }
        other => panic!("expected Final, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. Provider-native format (Anthropic content array) is rejected
//    — the host must normalize before calling commit_llm_response
// ---------------------------------------------------------------------------

#[test]
fn anthropic_content_array_format_is_rejected() {
    let mut state = fresh_state();
    let response = serde_json::json!({
        "content": [
            {"type": "tool_use", "name": "calc", "input": {"x": 1}}
        ]
    })
    .to_string();

    // content is an array → parse_final_response returns None → Err
    let result = process_llm_response(&mut state, &response);
    assert!(
        result.is_err(),
        "provider-native Anthropic format must be rejected; host must normalize"
    );
}

// ---------------------------------------------------------------------------
// 7. OpenAI "choices" format is rejected
// ---------------------------------------------------------------------------

#[test]
fn openai_choices_format_is_rejected() {
    let mut state = fresh_state();
    let response = serde_json::json!({
        "choices": [{
            "message": {
                "tool_calls": [{
                    "function": {"name": "weather.get", "arguments": "{\"city\":\"Jakarta\"}"}
                }]
            }
        }]
    })
    .to_string();

    let result = process_llm_response(&mut state, &response);
    assert!(
        result.is_err(),
        "provider-native OpenAI format must be rejected; host must normalize"
    );
}

// ---------------------------------------------------------------------------
// 8. step counter increments only on tool calls, not on final responses
// ---------------------------------------------------------------------------

#[test]
fn step_counter_increments_on_tool_call_not_final() {
    let mut state = fresh_state();
    assert_eq!(state.current_step, 0);

    let tool_call = serde_json::json!({
        "action": "call_tool", "tool": "t", "input": {}
    })
    .to_string();
    process_llm_response(&mut state, &tool_call).unwrap();
    assert_eq!(state.current_step, 1, "step should increment on tool call");

    let final_resp = serde_json::json!({ "response": "done" }).to_string();
    process_llm_response(&mut state, &final_resp).unwrap();
    assert_eq!(
        state.current_step, 1,
        "step must NOT increment on final response"
    );
}
