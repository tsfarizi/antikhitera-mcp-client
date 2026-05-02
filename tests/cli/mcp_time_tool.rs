use antikythera_cli::domain::use_cases::mcp_time_tool::{
    dispatch_mcp_tool, execute_mcp_get_current_time, mcp_time_tool_definition,
};
use serde_json::{Value, json};

#[test]
fn tool_definition_has_correct_name() {
    let def = mcp_time_tool_definition();
    assert_eq!(def["name"], "mcp_get_current_time");
    assert!(def["description"].as_str().is_some());
    assert!(def["parameters"].as_array().is_some());
}

#[test]
fn execute_returns_success_with_unix_time() {
    let result = execute_mcp_get_current_time().expect("should succeed");
    assert_eq!(result["status"], "success");
    let time = result["time"].as_u64().expect("time must be u64");
    assert!(time > 1_700_000_000, "unix time looks wrong: {time}");
}

#[test]
fn dispatch_known_tool_returns_json_string() {
    let output =
        dispatch_mcp_tool("mcp_get_current_time", &json!({})).expect("dispatch should succeed");
    let parsed: Value = serde_json::from_str(&output).expect("output must be valid JSON");
    assert_eq!(parsed["status"], "success");
    assert!(parsed["time"].is_number());
}

#[test]
fn dispatch_unknown_tool_returns_error() {
    let err = dispatch_mcp_tool("unknown_tool", &json!({}));
    assert!(err.is_err());
    let msg = format!("{:?}", err.unwrap_err());
    assert!(msg.contains("unknown MCP tool"));
}
