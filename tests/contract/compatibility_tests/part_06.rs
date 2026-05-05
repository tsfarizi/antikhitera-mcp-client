#[test]
fn tool_call_envelope_new() {
    let call = ToolCallEnvelope::new("search", serde_json::json!({"query": "test"}));
    assert_eq!(call.tool_name, "search");
    assert_eq!(
        call.input.get("query").and_then(|v| v.as_str()),
        Some("test")
    );
}

#[test]
fn tool_call_envelope_validate_empty_name() {
    let call = ToolCallEnvelope::new("", serde_json::json!({}));
    assert!(call.validate().is_err());
}

#[test]
fn tool_call_envelope_validate_valid() {
    let call = ToolCallEnvelope::new("tool", serde_json::json!({}));
    assert!(call.validate().is_ok());
}

#[test]
fn tool_call_envelope_required_field_present() {
    let call = ToolCallEnvelope::new("tool", serde_json::json!({"key": "value"}));
    let value = call
        .required_field("key")
        .and_then(|v| {
            v.as_str()
                .ok_or_else(|| "not a string".to_string())
                .map(|s| s.to_string())
        })
        .expect("field should exist");
    assert_eq!(value, "value");
}

#[test]
fn tool_call_envelope_required_field_missing() {
    let call = ToolCallEnvelope::new("tool", serde_json::json!({}));
    assert!(call.required_field("missing").is_err());
}

#[test]
fn tool_call_envelope_optional_field() {
    let call = ToolCallEnvelope::new("tool", serde_json::json!({"key": "value"}));
    let opt_value = call
        .optional_field("key")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    assert_eq!(opt_value.as_deref(), Some("value"));
    assert_eq!(call.optional_field("missing"), None);
}

#[test]
fn tool_result_envelope_success() {
    let result = ToolResultEnvelope::success("done");
    assert_eq!(result.outcome, ResultOutcome::Success);
    assert!(result.is_success());
    assert!(!result.is_failed());
    assert_eq!(result.error_text(), None);
}

#[test]
fn tool_result_envelope_error() {
    let result = ToolResultEnvelope::error("something went wrong");
    assert_eq!(result.outcome, ResultOutcome::Error);
    assert!(!result.is_success());
    assert!(result.is_failed());
    assert_eq!(result.error_text(), Some("something went wrong"));
}

#[test]
fn tool_result_envelope_partial_failure() {
    let result = ToolResultEnvelope::partial_failure("partial data", "some error");
    assert_eq!(result.outcome, ResultOutcome::PartialFailure);
    assert!(!result.is_success());
    assert!(result.is_failed());
}

#[test]
fn tool_execution_error_is_retryable() {
    assert!(
        ToolExecutionError::Timeout {
            tool_name: "search".to_string()
        }
        .is_retryable()
    );

    assert!(
        ToolExecutionError::Transient {
            message: "temp error".to_string()
        }
        .is_retryable()
    );

    assert!(
        !ToolExecutionError::ToolNotFound {
            tool_name: "bad".to_string()
        }
        .is_retryable()
    );
}

#[test]
fn tool_execution_error_message() {
    let err = ToolExecutionError::ExecutionFailed {
        tool_name: "search".to_string(),
        message: "network error".to_string(),
    };
    assert!(err.message().contains("search"));
    assert!(err.message().contains("network error"));
}

#[test]
fn contract_validator_call_empty_name() {
    let call = ToolCallEnvelope::new("", serde_json::json!({}));
    assert!(ContractValidator::validate_call(&call).is_err());
}

#[test]
fn contract_validator_call_valid() {
    let call = ToolCallEnvelope::new("tool", serde_json::json!({}));
    assert!(ContractValidator::validate_call(&call).is_ok());
}

#[test]
fn contract_validator_result_error_without_message() {
    let result = ToolResultEnvelope {
        outcome: ResultOutcome::Error,
        content: String::new(),
        error_message: None,
    };
    assert!(ContractValidator::validate_result("tool", &result).is_err());
}

#[test]
fn contract_validator_result_valid() {
    let result = ToolResultEnvelope::success("done");
    assert!(ContractValidator::validate_result("tool", &result).is_ok());
}

#[test]
fn contract_validator_result_to_error_success() {
    let result = ToolResultEnvelope::success("done");
    assert_eq!(ContractValidator::result_to_error("tool", &result), None);
}

#[test]
fn contract_validator_result_to_error_failed() {
    let result = ToolResultEnvelope::error("failed");
    let err = ContractValidator::result_to_error("tool", &result);
    assert!(err.is_some());
    assert!(matches!(
        err,
        Some(ToolExecutionError::ExecutionFailed { .. })
    ));
}

#[test]
fn tool_call_envelope_serialization() {
    let call = ToolCallEnvelope::new("search", serde_json::json!({"q": "test"}));
    let json = serde_json::to_string(&call).expect("serialize failed");
    let deserialized: ToolCallEnvelope =
        serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(deserialized, call);
}

#[test]
fn tool_result_envelope_serialization() {
    let result = ToolResultEnvelope::partial_failure("data", "err");
    let json = serde_json::to_string(&result).expect("serialize failed");
    let deserialized: ToolResultEnvelope =
        serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(deserialized, result);
}

#[test]
fn tool_execution_error_serialization() {
    let err = ToolExecutionError::ExecutionFailed {
        tool_name: "search".to_string(),
        message: "failed".to_string(),
    };
    let json = serde_json::to_string(&err).expect("serialize failed");
    let deserialized: ToolExecutionError =
        serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(deserialized, err);
}

#[test]
fn validate_tool_name_valid() {
    assert!(validate_tool_name("get_weather").is_ok());
    assert!(validate_tool_name("DATA_EXPORT_v2").is_ok());
    assert!(validate_tool_name("admin.tools.list").is_ok());
    assert!(validate_tool_name("getUser").is_ok());
    assert!(validate_tool_name("a").is_ok());
}

#[test]
fn validate_tool_name_empty() {
    assert!(validate_tool_name("").is_err());
}

#[test]
fn validate_tool_name_too_long() {
    let long_name = "a".repeat(129);
    assert!(validate_tool_name(&long_name).is_err());
}

#[test]
fn validate_tool_name_max_length() {
    let max_name = "a".repeat(128);
    assert!(validate_tool_name(&max_name).is_ok());
}

#[test]
fn validate_tool_name_invalid_characters() {
    assert!(validate_tool_name("get weather").is_err());
    assert!(validate_tool_name("tool,name").is_err());
    assert!(validate_tool_name("tool!").is_err());
    assert!(validate_tool_name("tool@name").is_err());
}
