#[test]
fn validate_tool_call_rejects_missing_required_param() {
    let registry = make_weather_registry();
    let args = serde_json::json!({}); // missing 'city'
    let err = registry.validate_call("weather.get", &args).unwrap_err();
    assert_eq!(
        err,
        ToolValidationError::MissingRequiredParam {
            tool: "weather.get".to_string(),
            param: "city".to_string(),
        }
    );
}


#[test]
fn validate_tool_call_passes_with_all_required_params() {
    let registry = make_weather_registry();
    let args = serde_json::json!({"city": "Jakarta"});
    assert!(registry.validate_call("weather.get", &args).is_ok());
}


#[test]
fn validate_tool_call_skips_when_registry_empty() {
    let empty_registry = ToolRegistry::default();
    let args = serde_json::json!({});
    // validate_tool_call (not validate_call) returns Ok when registry is empty
    assert!(validate_tool_call(&empty_registry, "any.tool", &args).is_ok());
}

// ---------------------------------------------------------------------------
// 10. ToolRegistry -- to_prompt_block renders tool list correctly
// ---------------------------------------------------------------------------

