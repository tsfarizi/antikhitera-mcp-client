//! Input validation tests

use antikythera_core::security::config::ValidationConfig;
use antikythera_core::security::validation::{InputValidator, ValidationResult};

#[test]
fn test_validator_creation() {
    let config = ValidationConfig::default();
    let validator = InputValidator::new(config).unwrap();
    assert_eq!(validator.config().max_input_size_bytes, 10 * 1024 * 1024);
}

#[test]
fn test_validate_size_valid() {
    let validator = InputValidator::from_config().unwrap();
    let result = validator.validate_size("small input");
    assert!(matches!(result, ValidationResult::Valid));
}

#[test]
fn test_validate_size_invalid() {
    let validator = InputValidator::from_config().unwrap();
    let large_input = "x".repeat(11 * 1024 * 1024);
    let result = validator.validate_size(&large_input);
    assert!(matches!(result, ValidationResult::Invalid(_)));
}

#[test]
fn test_validate_message_length_valid() {
    let validator = InputValidator::from_config().unwrap();
    let message = "a".repeat(1000);
    let result = validator.validate_message_length(&message);
    assert!(matches!(result, ValidationResult::Valid));
}

#[test]
fn test_validate_message_length_invalid() {
    let validator = InputValidator::from_config().unwrap();
    let message = "a".repeat(200_000);
    let result = validator.validate_message_length(&message);
    assert!(matches!(result, ValidationResult::Invalid(_)));
}

#[test]
fn test_validate_url_valid() {
    let validator = InputValidator::from_config().unwrap();
    let urls = vec![
        "https://example.com",
        "https://api.example.com/v1/endpoint",
        "https://localhost.local", // This has a TLD
    ];

    for url in urls {
        let result = validator.validate_url(url);
        assert!(matches!(result, ValidationResult::Valid), "URL {} should be valid", url);
    }
}

#[test]
fn test_validate_url_blocked() {
    let validator = InputValidator::from_config().unwrap();
    let blocked_urls = vec![
        "file:///etc/passwd",
        "data:text/html,<script>alert('xss')</script>",
    ];

    for url in blocked_urls {
        let result = validator.validate_url(url);
        assert!(matches!(result, ValidationResult::Invalid(_)), "URL {} should be blocked", url);
    }
}

#[test]
fn test_check_blocked_keywords() {
    let validator = InputValidator::from_config().unwrap();

    let valid_inputs = vec![
        "normal text",
        "This is a regular message",
        "No special characters here",
    ];

    for input in valid_inputs {
        let result = validator.check_blocked_keywords(input);
        assert!(matches!(result, ValidationResult::Valid), "Input '{}' should be valid", input);
    }

    let blocked_inputs = vec![
        "<script>alert('xss')</script>",
        "javascript:void(0)",
        "eval(malicious_code)",
    ];

    for input in blocked_inputs {
        let result = validator.check_blocked_keywords(input);
        assert!(matches!(result, ValidationResult::Invalid(_)), "Input '{}' should be blocked", input);
    }
}

#[test]
fn test_sanitize_html() {
    let validator = InputValidator::from_config().unwrap();

    let html = "<script>alert('xss')</script><div onclick=\"alert('click')\">content</div>";
    let sanitized = validator.sanitize_html(html);

    assert!(!sanitized.contains("<script>"));
    assert!(!sanitized.contains("javascript:"));
    assert!(!sanitized.contains("onclick="));
}

#[test]
fn test_validate_json_valid() {
    let validator = InputValidator::from_config().unwrap();

    let valid_jsons = vec![
        r#"{"key": "value"}"#,
        r#"{"array": [1, 2, 3]}"#,
        r#"{"nested": {"key": "value"}}"#,
    ];

    for json in valid_jsons {
        let result = validator.validate_json(json);
        assert!(result.is_ok(), "JSON '{}' should be valid", json);
    }
}

#[test]
fn test_validate_json_invalid() {
    let validator = InputValidator::from_config().unwrap();

    let invalid_jsons = vec![
        r#"{"key": "value""#,
        r#"not json at all"#,
        r#"{"unclosed": [1, 2, 3}"#,
    ];

    for json in invalid_jsons {
        let result = validator.validate_json(json);
        assert!(result.is_err(), "JSON '{}' should be invalid", json);
    }
}

#[test]
fn test_validate_json_nesting_depth() {
    let config = ValidationConfig {
        max_json_nesting_depth: 3,
        ..Default::default()
    };
    let validator = InputValidator::new(config).unwrap();

    let shallow_json = r#"{"level1": {"level2": {"level3": "value"}}}"#;
    assert!(validator.validate_json(shallow_json).is_ok());

    let deep_json = r#"{"l1": {"l2": {"l3": {"l4": {"l5": "value"}}}}}"#;
    assert!(validator.validate_json(deep_json).is_err());
}

#[test]
fn test_validate_json_array_length() {
    let config = ValidationConfig {
        max_json_array_length: 5,
        ..Default::default()
    };
    let validator = InputValidator::new(config).unwrap();

    let short_array = r#"{"array": [1, 2, 3]}"#;
    assert!(validator.validate_json(short_array).is_ok());

    let long_array = r#"{"array": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}"#;
    assert!(validator.validate_json(long_array).is_err());
}

#[test]
fn test_validate_tool_input() {
    let validator = InputValidator::from_config().unwrap();

    let valid_input = r#"{"param": "value", "url": "https://api.example.com"}"#;
    let result = validator.validate_tool_input("test_tool", valid_input);
    assert!(matches!(result, ValidationResult::Valid));

    let invalid_input = r#"{"param": "<script>alert('xss')</script>"}"#;
    let result = validator.validate_tool_input("test_tool", invalid_input);
    assert!(matches!(result, ValidationResult::Invalid(_)));
}

#[test]
fn test_validate_concurrent_calls() {
    let config = ValidationConfig {
        max_concurrent_tool_calls: 5,
        ..Default::default()
    };
    let validator = InputValidator::new(config).unwrap();

    assert!(matches!(validator.validate_concurrent_calls(3), ValidationResult::Valid));
    assert!(matches!(validator.validate_concurrent_calls(4), ValidationResult::Valid));
    assert!(matches!(validator.validate_concurrent_calls(5), ValidationResult::Invalid(_)));
}

#[test]
fn test_comprehensive_validation() {
    let validator = InputValidator::from_config().unwrap();

    let valid_input = "This is a normal message with no issues";
    assert!(validator.validate(valid_input).is_ok());

    let large_input = "x".repeat(200_000);
    assert!(validator.validate(&large_input).is_err());

    let malicious_input = "Check out this <script>alert('xss')</script>";
    assert!(validator.validate(malicious_input).is_err());
}

#[test]
fn test_update_config() {
    let mut validator = InputValidator::from_config().unwrap();

    let new_config = ValidationConfig {
        max_input_size_bytes: 5 * 1024 * 1024,
        max_message_length: 50_000,
        ..Default::default()
    };

    validator.update_config(new_config).unwrap();
    assert_eq!(validator.config().max_input_size_bytes, 5 * 1024 * 1024);
    assert_eq!(validator.config().max_message_length, 50_000);
}