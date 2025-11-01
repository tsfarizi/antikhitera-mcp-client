use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolInvokeError {
    #[error("MCP server '{server}' is not configured")]
    NotConfigured { server: String },
    #[error("failed to spawn MCP server '{server}': {source}")]
    Spawn {
        server: String,
        #[source]
        source: std::io::Error,
    },
    #[error("MCP server '{server}' transport error: {message}")]
    Transport { server: String, message: String },
    #[error("MCP server '{server}' returned invalid JSON: {source}")]
    InvalidJson {
        server: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("MCP server '{server}' returned JSON-RPC error {code}: {message}")]
    Rpc {
        server: String,
        code: i64,
        message: String,
    },
    #[error("MCP server '{server}' terminated unexpectedly")]
    Terminated { server: String },
    #[error("MCP server '{server}' request cancelled")]
    Cancelled { server: String },
}
