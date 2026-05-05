use antikythera_core::application::tooling::{
    BuiltinTransport, McpTransport, ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution,
    transport::{BuiltinToolFn, validate_arguments},
};
use serde_json::{Value, json};

// -- validation tests ---------------------------------------------------

#[test]
fn test_validate_arguments_missing_required() {
    let schema = json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    });
    let args = json!({});
    let err = validate_arguments(&schema, &args).unwrap_err();
    assert!(err.contains("missing required parameter 'city'"));
}

#[test]
fn test_validate_arguments_type_mismatch() {
    let schema = json!({
        "type": "object",
        "properties": {
            "count": { "type": "number" }
        }
    });
    let args = json!({"count": "not_a_number"});
    let err = validate_arguments(&schema, &args).unwrap_err();
    assert!(err.contains("must be of type 'number'"));
}

#[test]
fn test_validate_arguments_additional_properties_rejected() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "additionalProperties": false
    });
    let args = json!({"name": "test", "extra": true});
    let err = validate_arguments(&schema, &args).unwrap_err();
    assert!(err.contains("unexpected parameter 'extra'"));
}

#[test]
fn test_validate_arguments_valid() {
    let schema = json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" },
            "days": { "type": "number" }
        },
        "required": ["city"]
    });
    let args = json!({"city": "Jakarta", "days": 7});
    assert!(validate_arguments(&schema, &args).is_ok());
}

#[test]
fn test_validate_arguments_no_schema_accepts_any() {
    assert!(validate_arguments(&Value::Null, &json!({"any": "thing"})).is_ok());
}

#[test]
fn test_validate_arguments_empty_schema_accepts_empty_object() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false
    });
    assert!(validate_arguments(&schema, &json!({})).is_ok());
    assert!(validate_arguments(&schema, &json!(null)).is_err());
}

// -- transport tests ----------------------------------------------------

fn make_echo_tool() -> (Vec<ServerToolInfo>, BuiltinToolFn) {
    let tool = ServerToolInfo {
        name: "echo".to_string(),
        title: Some("Echo Tool".to_string()),
        description: Some("Returns the input unchanged".to_string()),
        icons: None,
        input_schema: Some(json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Text to echo" }
            },
            "required": ["message"],
            "additionalProperties": false
        })),
        output_schema: Some(json!({
            "type": "object",
            "properties": {
                "echo": { "type": "string" }
            },
            "required": ["echo"]
        })),
        annotations: Some(ToolAnnotations {
            audience: Some(vec!["assistant".to_string()]),
            priority: Some(0.5),
            last_modified: None,
        }),
        execution: Some(ToolExecution {
            task_support: Some(TaskSupport::Forbidden),
        }),
    };

    let handler: BuiltinToolFn = |args: &Value| {
        let msg = args
            .get("message")
            .and_then(Value::as_str)
            .ok_or("missing 'message' parameter")?;
        Ok(json!({ "echo": msg }))
    };

    (vec![tool], handler)
}

#[tokio::test]
async fn test_list_tools() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    let listed = transport.list_tools().await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "echo");
}

#[tokio::test]
async fn test_call_tool_success() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    let result = transport
        .call_tool("echo", json!({"message": "hello"}))
        .await
        .unwrap();
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    assert!(!is_error);
    let sc = result.get("structuredContent").unwrap();
    assert_eq!(sc.get("echo").and_then(Value::as_str), Some("hello"));
}

#[tokio::test]
async fn test_call_tool_input_validation_missing_required() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    // Missing required "message"
    let result = transport.call_tool("echo", json!({})).await.unwrap();
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(is_error, "missing required param must return isError: true");
    let text = result
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|b| b.get("text"))
        .and_then(Value::as_str)
        .unwrap();
    assert!(text.contains("missing required parameter 'message'"));
}

#[tokio::test]
async fn test_call_tool_input_validation_extra_param() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    // Extra unknown parameter
    let result = transport
        .call_tool("echo", json!({"message": "hi", "unknown": 42}))
        .await
        .unwrap();
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(is_error, "extra param must return isError: true");
}

#[tokio::test]
async fn test_call_tool_input_validation_type_mismatch() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    // "message" should be string, not number
    let result = transport
        .call_tool("echo", json!({"message": 123}))
        .await
        .unwrap();
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(is_error, "type mismatch must return isError: true");
}

#[tokio::test]
async fn test_unknown_tool() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    let result = transport.call_tool("nonexistent", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_custom_instructions() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools)
        .with_handler("echo", handler)
        .with_instructions("Custom guidance");
    assert_eq!(
        transport.instructions().await,
        Some("Custom guidance".to_string())
    );
}

#[tokio::test]
async fn test_invalid_tool_name_skipped() {
    let bad_tool = ServerToolInfo {
        name: "bad name with spaces".to_string(),
        title: None,
        description: None,
        icons: None,
        input_schema: None,
        output_schema: None,
        annotations: None,
        execution: None,
    };
    let transport = BuiltinTransport::with_tools("test", vec![bad_tool]);
    assert!(transport.list_tools().await.is_empty());
}

#[tokio::test]
async fn test_tool_metadata() {
    let (tools, handler) = make_echo_tool();
    let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
    let meta = transport.tool_metadata("echo").await.unwrap();
    assert_eq!(meta.name, "echo");
    assert!(meta.input_schema.is_some());
    assert!(meta.output_schema.is_some());
}

#[tokio::test]
async fn test_empty_tools() {
    let transport = BuiltinTransport::with_tools("test", vec![]);
    assert!(transport.list_tools().await.is_empty());
    assert_eq!(
        transport.instructions().await,
        Some("No built-in tools are currently configured.".to_string())
    );
}
