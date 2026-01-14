//! MCP Transport Abstraction Layer
//!
//! This module provides transport abstraction for MCP communication,
//! supporting both STDIO (subprocess) and HTTP transports.

mod http;

use async_trait::async_trait;
use serde_json::Value;

use super::error::ToolInvokeError;
use super::interface::ServerToolInfo;

pub use http::{HttpTransport, HttpTransportConfig, TransportMode};

/// Transport trait for MCP communication.
///
/// Implementations handle the low-level communication with MCP servers,
/// whether via STDIO (subprocess) or HTTP.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Connect to the server and perform initialization handshake.
    async fn connect(&self) -> Result<(), ToolInvokeError>;

    /// Send a JSON-RPC request and wait for response.
    async fn send_request(&self, method: &str, params: Value) -> Result<Value, ToolInvokeError>;

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolInvokeError>;

    /// Call a tool on the server.
    async fn call_tool(&self, tool: &str, arguments: Value) -> Result<Value, ToolInvokeError>;

    /// Get server instructions (from initialize response).
    async fn instructions(&self) -> Option<String>;

    /// Get tool metadata from cache.
    async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo>;

    /// Get server name.
    fn server_name(&self) -> &str;

    /// Check if the transport is connected.
    async fn is_connected(&self) -> bool;

    /// Disconnect from the server.
    async fn disconnect(&self);
}
