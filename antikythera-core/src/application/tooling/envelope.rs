use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Canonical MCP tool call envelope used by the runtime boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEnvelope {
    pub tool: String,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default)]
    pub correlation_id: Option<String>,
}

/// Canonical MCP tool result envelope used by the runtime boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultEnvelope {
    pub tool: String,
    pub success: bool,
    #[serde(default)]
    pub output: Value,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    MissingTool,
    InvalidArguments,
    InconsistentResult,
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::MissingTool => write!(f, "tool must be a non-empty string"),
            EnvelopeError::InvalidArguments => write!(f, "arguments must be a JSON object"),
            EnvelopeError::InconsistentResult => {
                write!(f, "error must be present for failed results and absent for successful ones")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Validate strict tool-call envelope contract.
pub fn validate_tool_call_envelope(env: &ToolCallEnvelope) -> Result<(), EnvelopeError> {
    if env.tool.trim().is_empty() {
        return Err(EnvelopeError::MissingTool);
    }
    if !env.arguments.is_object() {
        return Err(EnvelopeError::InvalidArguments);
    }
    Ok(())
}

/// Validate strict tool-result envelope contract.
pub fn validate_tool_result_envelope(env: &ToolResultEnvelope) -> Result<(), EnvelopeError> {
    if env.tool.trim().is_empty() {
        return Err(EnvelopeError::MissingTool);
    }

    match (env.success, env.error.as_ref().map(|s| s.trim().is_empty())) {
        (true, Some(false)) => Err(EnvelopeError::InconsistentResult),
        (false, None) | (false, Some(true)) => Err(EnvelopeError::InconsistentResult),
        _ => Ok(()),
    }
}
