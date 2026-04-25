//! Input Validation and Sanitization
//!
//! Comprehensive input validation for WASM components and user inputs.

use super::config::ValidationConfig;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
    Sanitized(String),
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Input validator
pub struct InputValidator {
    config: ValidationConfig,
    allowed_url_regexes: Vec<Regex>,
    blocked_url_regexes: Vec<Regex>,
    blocked_keywords_set: HashSet<String>,
}

impl InputValidator {
    pub fn new(config: ValidationConfig) -> Result<Self, String> {
        let allowed_url_regexes = config
            .allowed_url_patterns
            .iter()
            .map(|pattern| Regex::new(pattern).map_err(|e| format!("Invalid allowed URL pattern: {}", e)))
            .collect::<Result<Vec<_>, _>>()?;

        let blocked_url_regexes = config
            .blocked_url_patterns
            .iter()
            .map(|pattern| Regex::new(pattern).map_err(|e| format!("Invalid blocked URL pattern: {}", e)))
            .collect::<Result<Vec<_>, _>>()?;

        let blocked_keywords_set = config
            .blocked_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        Ok(Self {
            config,
            allowed_url_regexes,
            blocked_url_regexes,
            blocked_keywords_set,
        })
    }

    pub fn from_config() -> Result<Self, String> {
        Self::new(ValidationConfig::default())
    }

    /// Validate input size
    pub fn validate_size(&self, input: &str) -> ValidationResult {
        let size = input.len() as u64;
        if size > self.config.max_input_size_bytes {
            return ValidationResult::Invalid(format!(
                "Input size {} bytes exceeds maximum {} bytes",
                size, self.config.max_input_size_bytes
            ));
        }
        ValidationResult::Valid
    }

    /// Validate message length
    pub fn validate_message_length(&self, message: &str) -> ValidationResult {
        let length = message.chars().count();
        if length > self.config.max_message_length {
            return ValidationResult::Invalid(format!(
                "Message length {} exceeds maximum {}",
                length, self.config.max_message_length
            ));
        }
        ValidationResult::Valid
    }

    /// Validate URL
    pub fn validate_url(&self, url: &str) -> ValidationResult {
        // Check blocked patterns first
        for regex in &self.blocked_url_regexes {
            if regex.is_match(url) {
                return ValidationResult::Invalid(format!("URL matches blocked pattern: {}", url));
            }
        }

        // Check allowed patterns
        let is_allowed = self.allowed_url_regexes.iter().any(|regex| regex.is_match(url));
        if !is_allowed {
            return ValidationResult::Invalid(format!("URL does not match any allowed pattern: {}", url));
        }

        ValidationResult::Valid
    }

    /// Check for blocked keywords
    pub fn check_blocked_keywords(&self, input: &str) -> ValidationResult {
        let lower_input = input.to_lowercase();
        for keyword in &self.blocked_keywords_set {
            if lower_input.contains(keyword) {
                return ValidationResult::Invalid(format!("Input contains blocked keyword: {}", keyword));
            }
        }
        ValidationResult::Valid
    }

    /// Sanitize HTML content
    pub fn sanitize_html(&self, html: &str) -> String {
        if !self.config.sanitize_html {
            return html.to_string();
        }

        // Basic HTML sanitization - remove script tags and event handlers
        let sanitized = html
            .replace("<script", "")
            .replace("</script>", "")
            .replace("javascript:", "")
            .replace("onerror=", "")
            .replace("onload=", "")
            .replace("onclick=", "");

        sanitized
    }

    /// Validate JSON structure
    pub fn validate_json(&self, json_str: &str) -> Result<Value, String> {
        if !self.config.validate_json_schema {
            return serde_json::from_str(json_str).map_err(|e| e.to_string());
        }

        let value: Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;

        self.validate_json_structure(&value, 0)?;

        Ok(value)
    }

    /// Validate JSON structure recursively
    fn validate_json_structure(&self, value: &Value, depth: u32) -> Result<(), String> {
        if depth > self.config.max_json_nesting_depth {
            return Err(format!("JSON nesting depth {} exceeds maximum {}", depth, self.config.max_json_nesting_depth));
        }

        match value {
            Value::Array(arr) => {
                if arr.len() as u32 > self.config.max_json_array_length {
                    return Err(format!("JSON array length {} exceeds maximum {}", arr.len(), self.config.max_json_array_length));
                }
                for item in arr {
                    self.validate_json_structure(item, depth + 1)?;
                }
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    self.validate_json_structure(v, depth + 1)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate tool call input
    pub fn validate_tool_input(&self, _tool_name: &str, input: &str) -> ValidationResult {
        // Validate input size
        if let ValidationResult::Invalid(msg) = self.validate_size(input) {
            return ValidationResult::Invalid(msg);
        }

        // Validate JSON structure
        if let Err(msg) = self.validate_json(input) {
            return ValidationResult::Invalid(format!("Invalid JSON in tool input: {}", msg));
        }

        // Check for blocked keywords
        if let ValidationResult::Invalid(msg) = self.check_blocked_keywords(input) {
            return ValidationResult::Invalid(msg);
        }

        // Validate URLs in input
        if let Ok(json) = self.validate_json(input) {
            self.validate_urls_in_json(&json);
        }

        ValidationResult::Valid
    }

    /// Validate URLs in JSON structure
    fn validate_urls_in_json(&self, value: &Value) -> ValidationResult {
        match value {
            Value::String(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    if let ValidationResult::Invalid(msg) = self.validate_url(s) {
                        return ValidationResult::Invalid(msg);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    if let ValidationResult::Invalid(msg) = self.validate_urls_in_json(item) {
                        return ValidationResult::Invalid(msg);
                    }
                }
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    if let ValidationResult::Invalid(msg) = self.validate_urls_in_json(v) {
                        return ValidationResult::Invalid(msg);
                    }
                }
            }
            _ => {}
        }
        ValidationResult::Valid
    }

    /// Validate concurrent tool calls
    pub fn validate_concurrent_calls(&self, current_calls: u32) -> ValidationResult {
        if current_calls >= self.config.max_concurrent_tool_calls {
            return ValidationResult::Invalid(format!(
                "Concurrent tool calls {} exceeds maximum {}",
                current_calls, self.config.max_concurrent_tool_calls
            ));
        }
        ValidationResult::Valid
    }

    /// Comprehensive validation
    pub fn validate(&self, input: &str) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate size
        match self.validate_size(input) {
            ValidationResult::Invalid(msg) => errors.push(ValidationError {
                field: "size".to_string(),
                message: msg,
            }),
            _ => {}
        }

        // Validate message length
        match self.validate_message_length(input) {
            ValidationResult::Invalid(msg) => errors.push(ValidationError {
                field: "message_length".to_string(),
                message: msg,
            }),
            _ => {}
        }

        // Check blocked keywords
        match self.check_blocked_keywords(input) {
            ValidationResult::Invalid(msg) => errors.push(ValidationError {
                field: "keywords".to_string(),
                message: msg,
            }),
            _ => {}
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ValidationConfig) -> Result<(), String> {
        let allowed_url_patterns = config.allowed_url_patterns.clone();
        let blocked_url_patterns = config.blocked_url_patterns.clone();
        let blocked_keywords = config.blocked_keywords.clone();

        self.config = config;
        self.allowed_url_regexes = allowed_url_patterns
            .iter()
            .map(|pattern| Regex::new(pattern).map_err(|e| format!("Invalid allowed URL pattern: {}", e)))
            .collect::<Result<Vec<_>, _>>()?;

        self.blocked_url_regexes = blocked_url_patterns
            .iter()
            .map(|pattern| Regex::new(pattern).map_err(|e| format!("Invalid blocked URL pattern: {}", e)))
            .collect::<Result<Vec<_>, _>>()?;

        self.blocked_keywords_set = blocked_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_size() {
        let validator = InputValidator::from_config().unwrap();
        assert!(matches!(validator.validate_size("small"), ValidationResult::Valid));

        let large_input = "x".repeat(11 * 1024 * 1024);
        assert!(matches!(validator.validate_size(&large_input), ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validate_url() {
        let validator = InputValidator::from_config().unwrap();
        assert!(matches!(validator.validate_url("https://example.com"), ValidationResult::Valid));
        assert!(matches!(validator.validate_url("file://etc/passwd"), ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_check_blocked_keywords() {
        let validator = InputValidator::from_config().unwrap();
        assert!(matches!(validator.check_blocked_keywords("normal text"), ValidationResult::Valid));
        assert!(matches!(validator.check_blocked_keywords("<script>alert('xss')</script>"), ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_sanitize_html() {
        let validator = InputValidator::from_config().unwrap();
        let html = "<script>alert('xss')</script>";
        let sanitized = validator.sanitize_html(html);
        assert!(!sanitized.contains("<script>"));
    }

    #[test]
    fn test_validate_json() {
        let validator = InputValidator::from_config().unwrap();
        let valid_json = r#"{"key": "value"}"#;
        assert!(validator.validate_json(valid_json).is_ok());

        let invalid_json = r#"{"key": "value""#;
        assert!(validator.validate_json(invalid_json).is_err());
    }
}