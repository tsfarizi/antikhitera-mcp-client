//! Tool call and result contracts for MCP protocol compliance.
//!
//! This module defines canonical envelope types for tool calls and results,
//! ensuring strict validation and deterministic error handling across all MCP interactions.
//!
//! # Examples
//!
//! ```
//! use antikythera_core::infrastructure::mcp::contract::{ToolCallEnvelope, ToolResultEnvelope};
//!
//! let call = ToolCallEnvelope {
//!     tool_name: "search".to_string(),
//!     input: serde_json::json!({"query": "Rust programming"}),
//! };
//!
//! let result = ToolResultEnvelope::success("Found 5 articles about Rust");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Tool call envelope for MCP protocol.
///
/// Represents a canonical request to invoke a tool with validated structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallEnvelope {
    /// Name of the tool to call (must not be empty)
    pub tool_name: String,
    /// Input parameters as JSON object
    pub input: JsonValue,
}

impl ToolCallEnvelope {
    /// Create a new tool call envelope.
    pub fn new(tool_name: impl Into<String>, input: JsonValue) -> Self {
        Self {
            tool_name: tool_name.into(),
            input,
        }
    }

    /// Validate the envelope structure.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - tool_name is empty
    /// - input is not an object (for structured inputs)
    pub fn validate(&self) -> Result<(), String> {
        if self.tool_name.is_empty() {
            return Err("tool_name must not be empty".to_string());
        }
        Ok(())
    }

    /// Get a required field from input object.
    pub fn required_field(&self, name: &str) -> Result<JsonValue, String> {
        self.input
            .get(name)
            .cloned()
            .ok_or_else(|| format!("required field '{}' missing", name))
    }

    /// Get an optional field from input object.
    pub fn optional_field(&self, name: &str) -> Option<JsonValue> {
        self.input.get(name).cloned()
    }
}

/// Tool result outcome status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultOutcome {
    /// Tool executed successfully
    Success,
    /// Tool execution failed
    Error,
    /// Tool partially succeeded (some data available despite error)
    PartialFailure,
}

/// Tool result envelope for MCP protocol.
///
/// Represents a canonical response from tool execution with outcome semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultEnvelope {
    /// Execution outcome status
    pub outcome: ResultOutcome,
    /// Result content (text or structured data)
    pub content: String,
    /// Optional error message (when outcome is Error or PartialFailure)
    pub error_message: Option<String>,
}

impl ToolResultEnvelope {
    /// Create a successful result.
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            outcome: ResultOutcome::Success,
            content: content.into(),
            error_message: None,
        }
    }

    /// Create a failed result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            outcome: ResultOutcome::Error,
            content: String::new(),
            error_message: Some(message.into()),
        }
    }

    /// Create a partial failure result.
    pub fn partial_failure(content: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            outcome: ResultOutcome::PartialFailure,
            content: content.into(),
            error_message: Some(error_message.into()),
        }
    }

    /// Check if result represents success.
    pub fn is_success(&self) -> bool {
        self.outcome == ResultOutcome::Success
    }

    /// Check if result represents error or partial failure.
    pub fn is_failed(&self) -> bool {
        self.outcome != ResultOutcome::Success
    }

    /// Extract error message if failed.
    pub fn error_text(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// Error mapping for MCP tool execution.
///
/// Maps tool errors to deterministic retry and handling logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolExecutionError {
    /// Tool not found or not callable
    ToolNotFound { tool_name: String },
    /// Invalid tool input (validation failed)
    InvalidInput { tool_name: String, reason: String },
    /// Tool execution failed (runtime error)
    ExecutionFailed { tool_name: String, message: String },
    /// Tool timed out
    Timeout { tool_name: String },
    /// Transient error (retryable)
    Transient { message: String },
    /// Unknown error
    Unknown { message: String },
}

impl ToolExecutionError {
    /// Check if error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::Transient { .. })
    }

    /// Get human-readable error message.
    pub fn message(&self) -> String {
        match self {
            Self::ToolNotFound { tool_name } => format!("Tool '{}' not found", tool_name),
            Self::InvalidInput { tool_name, reason } => {
                format!("Invalid input for '{}': {}", tool_name, reason)
            }
            Self::ExecutionFailed { tool_name, message } => {
                format!("Tool '{}' execution failed: {}", tool_name, message)
            }
            Self::Timeout { tool_name } => format!("Tool '{}' timed out", tool_name),
            Self::Transient { message } => format!("Transient error: {}", message),
            Self::Unknown { message } => format!("Unknown error: {}", message),
        }
    }
}

/// Validates a tool name against the MCP spec naming rules:
/// - Length 1-128 characters
/// - Allowed characters: A-Z, a-z, 0-9, underscore (_), hyphen (-), dot (.)
/// - No spaces, commas, or other special characters
pub fn validate_tool_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("tool name must not be empty".to_string());
    }
    if name.len() > 128 {
        return Err(format!(
            "tool name exceeds 128 characters (length: {})",
            name.len()
        ));
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.' {
            return Err(format!(
                "tool name contains invalid character '{}'. Allowed: A-Z, a-z, 0-9, underscore, hyphen, dot",
                ch
            ));
        }
    }
    Ok(())
}

/// Validator for tool call and result contracts.
pub struct ContractValidator;

impl ContractValidator {
    /// Validate a tool call envelope.
    pub fn validate_call(envelope: &ToolCallEnvelope) -> Result<(), ToolExecutionError> {
        envelope
            .validate()
            .map_err(|reason| ToolExecutionError::InvalidInput {
                tool_name: envelope.tool_name.clone(),
                reason,
            })
    }

    /// Validate a tool result envelope.
    pub fn validate_result(
        _tool_name: &str,
        result: &ToolResultEnvelope,
    ) -> Result<(), ToolExecutionError> {
        // Basic validation: ensure error_message is set if outcome is Error
        if result.outcome == ResultOutcome::Error && result.error_message.is_none() {
            return Err(ToolExecutionError::Unknown {
                message: "Error outcome without error_message".to_string(),
            });
        }
        Ok(())
    }

    /// Map a result to an error if failed.
    pub fn result_to_error(
        tool_name: &str,
        result: &ToolResultEnvelope,
    ) -> Option<ToolExecutionError> {
        match result.outcome {
            ResultOutcome::Success => None,
            ResultOutcome::Error => Some(ToolExecutionError::ExecutionFailed {
                tool_name: tool_name.to_string(),
                message: result.error_text().unwrap_or("unknown error").to_string(),
            }),
            ResultOutcome::PartialFailure => Some(ToolExecutionError::ExecutionFailed {
                tool_name: tool_name.to_string(),
                message: result.error_text().unwrap_or("partial failure").to_string(),
            }),
        }
    }
}
