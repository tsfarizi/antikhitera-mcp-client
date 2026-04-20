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
// 5. Plain text (non-JSON) â†’ Final with string value
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
//    â€” the host must normalize before calling commit_llm_response
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

    // content is an array â†’ parse_final_response returns None â†’ Err
    let result = process_llm_response(&mut state, &response);
    assert!(
        result.is_err(),
        "provider-native Anthropic format must be rejected; host must normalize"
    );
}

// ---------------------------------------------------------------------------
// 7. OpenAI "choices" format is rejected
// ---------------------------------------------------------------------------

