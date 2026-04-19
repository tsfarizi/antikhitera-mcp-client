//! Agent directive types for parsing LLM responses.

use serde::Deserialize;
use serde_json::Value;

/// Directive extracted from LLM response.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentDirective {
    /// Final response to user
    Final { response: String },
    /// Call a single tool
    CallTool { tool: String, input: Value },
    /// Call multiple tools in parallel (reserved for future use)
    #[allow(dead_code)]
    CallTools(Vec<(String, Value)>),
}
