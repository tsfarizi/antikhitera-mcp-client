//! MCP Transport Abstraction Layer
//!
//! This module provides transport abstraction for MCP communication,
//! supporting both STDIO (subprocess) and HTTP transports.
//!
//! ## Module Structure
//!
//! - `config` - Transport configuration types (TransportMode, HttpTransportConfig)
//! - `http` - HTTP transport implementation
//!   - `sse` - SSE listener and endpoint resolution
//!   - `rpc` - JSON-RPC request/notification handling
//!   - `tools` - Tool cache management

mod config;
mod http;

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::error::ToolInvokeError;
use super::interface::ServerToolInfo;

// Re-export public types
pub use config::{HttpTransportConfig, TransportCapability, TransportMode};
pub use http::HttpTransport;

/// Transport trait for MCP communication.
///
/// Implementations handle the low-level communication with MCP servers,
/// whether via STDIO (subprocess) or HTTP.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

    /// List all tools from cache.
    async fn list_tools(&self) -> Vec<ServerToolInfo>;

    /// Get server name.
    fn server_name(&self) -> &str;

    /// Check if the transport is connected.
    async fn is_connected(&self) -> bool;

    /// Disconnect from the server.
    async fn disconnect(&self);
}

/// Factory contract for pluggable transport providers.
pub trait TransportFactory: Send + Sync {
    /// Stable factory identifier (e.g. `http`, `websocket`, `internal-bridge`).
    fn id(&self) -> &str;

    /// Capabilities supported by this transport implementation.
    fn capabilities(&self) -> Vec<TransportCapability>;

    /// Build a transport instance from a generic config JSON object.
    fn create(&self, config: Value) -> Result<Arc<dyn McpTransport>, ToolInvokeError>;
}

/// Registry for transport factories with capability-aware selection.
#[derive(Default)]
pub struct TransportRegistry {
    factories: Mutex<HashMap<String, Arc<dyn TransportFactory>>>,
}

impl TransportRegistry {
    /// Create an empty transport registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a transport factory.
    pub fn register(&self, factory: Arc<dyn TransportFactory>) {
        let mut factories = self.factories.lock().expect("transport registry lock");
        factories.insert(factory.id().to_string(), factory);
    }

    /// Return all registered transport IDs.
    pub fn list(&self) -> Vec<String> {
        let factories = self.factories.lock().expect("transport registry lock");
        factories.keys().cloned().collect()
    }

    /// Get a factory by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn TransportFactory>> {
        let factories = self.factories.lock().expect("transport registry lock");
        factories.get(id).cloned()
    }

    /// Select all transports that satisfy the required capabilities.
    pub fn select_by_capabilities(
        &self,
        required: &[TransportCapability],
    ) -> Vec<Arc<dyn TransportFactory>> {
        let factories = self.factories.lock().expect("transport registry lock");
        factories
            .values()
            .filter(|factory| {
                let offered = factory.capabilities();
                required.iter().all(|cap| offered.contains(cap))
            })
            .cloned()
            .collect()
    }
}
