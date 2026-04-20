// ---------------------------------------------------------------------------
// 8. Tool registry -- validation blocks unknown tool calls
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn unknown_tool_call_returns_error_when_registry_populated() {
    // Register only one known tool
    let tools_json = serde_json::json!([
        {
            "name": "weather.get",
            "description": "Get weather",
            "parameters": [
                {"name": "city", "param_type": "string", "description": "City", "required": true}
            ]
        }
    ])
    .to_string();
    register_tools(&tools_json).unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "book a flight")).unwrap();

    // LLM tries to call a tool not in the registry
    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "flights.book",
        "input": {"destination": "Tokyo"}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(result.is_err(), "should reject unknown tool call");
    let err = result.unwrap_err();
    assert!(
        err.contains("flights.book"),
        "error should mention the unknown tool name, got: {err}"
    );
}

