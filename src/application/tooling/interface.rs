use async_trait::async_trait;
use serde_json::Value;

use super::error::ToolInvokeError;

#[derive(Debug, Clone)]
pub struct ServerToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

#[async_trait]
pub trait ToolServerInterface: Send + Sync {
    async fn invoke_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError>;

    async fn server_instructions(&self, server: &str) -> Option<String>;

    async fn tool_metadata(&self, server: &str, tool: &str) -> Option<ServerToolInfo>;
}
