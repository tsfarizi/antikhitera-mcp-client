//! JSON Validator with Auto-Retry Mechanism
//!
//! Validates LLM responses against schema and auto-retries on failure.

use super::types::{JsonSchema, ValidationError};
use serde::{Deserialize, Serialize};

// ============================================================================
// Validation Result
// ============================================================================

/// Result of JSON validation attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Error message if validation failed
    pub error: Option<String>,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Final validated JSON
    pub json: Option<String>,
}

// ============================================================================
// Validator
// ============================================================================

/// JSON validator with retry support
#[derive(Debug, Clone)]
pub struct JsonValidator {
    /// Expected schema
    schema: JsonSchema,
    /// Maximum retry attempts
    max_retries: u32,
}

impl JsonValidator {
    /// Create new validator with schema
    pub fn new(schema: JsonSchema) -> Self {
        Self {
            schema,
            max_retries: 3,
        }
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Validate and auto-retry with error feedback
    ///
    /// Returns validation result with final JSON or error
    pub fn validate_with_retry(&self, initial_response: &str) -> ValidationResult {
        let mut current_response = initial_response.to_string();
        let mut retry_count = 0;

        loop {
            match self.schema.validate(&current_response) {
                Ok(()) => {
                    return ValidationResult {
                        valid: true,
                        error: None,
                        retry_count,
                        json: Some(current_response),
                    };
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count > self.max_retries {
                        return ValidationResult {
                            valid: false,
                            error: Some(format!("Failed after {} retries: {}", self.max_retries, e)),
                            retry_count,
                            json: None,
                        };
                    }

                    // Generate retry prompt with error feedback
                    current_response = self.generate_retry_prompt(&current_response, &e);

                    // In production, this would call LLM again with the retry prompt
                    // For now, return error - actual retry happens in FFI layer
                    return ValidationResult {
                        valid: false,
                        error: Some(e.to_string()),
                        retry_count,
                        json: Some(current_response),
                    };
                }
            }
        }
    }

    /// Generate retry prompt with specific error feedback
    fn generate_retry_prompt(&self, original_response: &str, error: &ValidationError) -> String {
        format!(
            "Your previous response failed validation.\n\n\
             Error: {}\n\n\
             Original response:\n\
             ```json\n\
             {}\n\
             ```\n\n\
             Please provide a corrected JSON response that matches the schema.\n\
             {}",
            error,
            original_response,
            self.schema.to_prompt_instruction()
        )
    }

    /// Get the schema prompt to append to main prompt
    pub fn get_schema_prompt(&self) -> String {
        self.schema.to_prompt_instruction()
    }
}

// ============================================================================
// Retry Manager (for tracking across calls)
// ============================================================================

/// Manages retry state across multiple LLM calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryManager {
    /// Current retry attempt
    pub current_attempt: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// History of validation errors
    pub error_history: Vec<String>,
}

impl RetryManager {
    /// Create new retry manager
    pub fn new(max_retries: u32) -> Self {
        Self {
            current_attempt: 0,
            max_retries,
            error_history: Vec::new(),
        }
    }

    /// Record a validation error
    pub fn record_error(&mut self, error: &ValidationError) {
        self.current_attempt += 1;
        self.error_history.push(error.to_string());
    }

    /// Check if retries are exhausted
    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.max_retries
    }

    /// Generate retry prompt
    pub fn retry_prompt(&self, schema_prompt: &str, last_response: &str) -> String {
        let errors_summary = self.error_history.iter()
            .enumerate()
            .map(|(i, e)| format!("Attempt {}: {}", i + 1, e))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "JSON validation failed after {} attempts.\n\n\
             Previous errors:\n\
             {}\n\n\
             Last response:\n\
             ```json\n\
             {}\n\
             ```\n\n\
             {}",
            self.current_attempt,
            errors_summary,
            last_response,
            schema_prompt
        )
    }
}
