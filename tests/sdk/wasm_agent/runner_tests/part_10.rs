// ---------------------------------------------------------------------------
// 9. Tool registry -- validation blocks missing required param
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn missing_required_param_returns_error() {
    // Use a unique tool name to avoid cross-test registry pollution
    let tools_json = serde_json::json!([
        {
            "name": "geo.lookup",
            "description": "Lookup geo coordinates",
            "parameters": [
                {"name": "lat", "param_type": "number", "description": "Latitude", "required": true},
                {"name": "lon", "param_type": "number", "description": "Longitude", "required": true}
            ]
        }
    ])
    .to_string();
    register_tools(&tools_json).unwrap();

    let session_id = init(&config_json()).unwrap();
    let prepared = prepare_user_turn(&prepare_request(&session_id, "find location")).unwrap();

    // LLM omits 'lon' (required)
    let response = serde_json::json!({
        "action": "call_tool",
        "tool": "geo.lookup",
        "input": {"lat": 1.28}
    })
    .to_string();

    let result = commit_llm_response(&prepared, &response);
    assert!(
        result.is_err(),
        "should reject call with missing required param"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("lon"),
        "error should mention the missing param, got: {err}"
    );
}

