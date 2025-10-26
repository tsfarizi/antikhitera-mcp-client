use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ToolDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServerGuidance {
    pub name: String,
    pub instruction: String,
}
