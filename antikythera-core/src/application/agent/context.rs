use serde::Serialize;
use serde_json::Value;

use crate::application::tooling::{ToolAnnotations, ToolExecution, ToolIcon};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ToolContext {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDescriptor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<ServerGuidance>,
}

impl ToolContext {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.servers.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<ToolIcon>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolExecution>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerGuidance {
    pub name: String,
    pub instruction: String,
}
