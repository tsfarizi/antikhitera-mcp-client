//! HTTP Transport for MCP servers.
//!
//! Implements JSON-RPC 2.0 over HTTP POST for MCP communication.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex as AsyncMutex;
use tracing::debug;

use super::McpTransport;
use crate::application::tooling::error::ToolInvokeError;
use crate::application::tooling::interface::ServerToolInfo;

const PROTOCOL_VERSION: &str = "2025-06-18";

/// HTTP Transport configuration.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Server name identifier
    pub name: String,
    /// Base URL for the MCP server
    pub url: String,
    /// Optional authorization headers
    pub headers: HashMap<String, String>,
}

/// HTTP Transport for MCP communication.
#[derive(Clone)]
pub struct HttpTransport {
    inner: Arc<HttpTransportInner>,
}

struct HttpTransportInner {
    config: HttpTransportConfig,
    client: Client,
    id_counter: AtomicU64,
    connected: AtomicBool,
    instructions: AsyncMutex<Option<String>>,
    tool_cache: AsyncMutex<HashMap<String, ServerToolInfo>>,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(config: HttpTransportConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            inner: Arc::new(HttpTransportInner {
                config,
                client,
                id_counter: AtomicU64::new(1),
                connected: AtomicBool::new(false),
                instructions: AsyncMutex::new(None),
                tool_cache: AsyncMutex::new(HashMap::new()),
            }),
        }
    }

    /// Get the RPC endpoint URL.
    fn rpc_url(&self) -> String {
        let base = self.inner.config.url.trim_end_matches('/');
        format!("{}/rpc", base)
    }

    /// Get the server name.
    pub fn get_name(&self) -> &str {
        &self.inner.config.name
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn connect(&self) -> Result<(), ToolInvokeError> {
        if self.inner.connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        debug!(
            server = %self.inner.config.name,
            url = %self.inner.config.url,
            "Connecting to HTTP MCP server"
        );

        // Send initialize request
        let params = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
                "title": "CBT MCP Client"
            },
            "capabilities": {}
        });

        let result = self.send_request("initialize", params).await?;

        // Extract instructions if present
        if let Some(text) = result.get("instructions").and_then(Value::as_str) {
            let mut instructions = self.inner.instructions.lock().await;
            *instructions = Some(text.to_string());
        }

        // Send initialized notification
        self.send_notification("notifications/initialized", json!({}))
            .await?;

        // Refresh tool cache
        self.refresh_tools().await?;

        self.inner.connected.store(true, Ordering::SeqCst);

        debug!(
            server = %self.inner.config.name,
            "Successfully connected to HTTP MCP server"
        );

        Ok(())
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value, ToolInvokeError> {
        let id = self.inner.id_counter.fetch_add(1, Ordering::SeqCst);
        let request_id = format!("req-{}", id);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        let url = self.rpc_url();
        debug!(
            server = %self.inner.config.name,
            method = method,
            "Sending HTTP JSON-RPC request"
        );

        let mut request = self
            .inner
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload);

        // Add custom headers
        for (key, value) in &self.inner.config.headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ToolInvokeError::Transport {
                server: self.inner.config.name.clone(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(ToolInvokeError::Transport {
                server: self.inner.config.name.clone(),
                message: format!("HTTP error: {}", response.status()),
            });
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| ToolInvokeError::Transport {
                server: self.inner.config.name.clone(),
                message: format!("Failed to parse JSON response: {}", e),
            })?;

        // Check for JSON-RPC error
        if let Some(error) = body.get("error").and_then(Value::as_object) {
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(-32000);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Unknown error")
                .to_string();
            return Err(ToolInvokeError::Rpc {
                server: self.inner.config.name.clone(),
                code,
                message,
            });
        }

        // Extract result
        let result = body.get("result").cloned().unwrap_or(Value::Null);
        Ok(result)
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolInvokeError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let url = self.rpc_url();

        let mut request = self
            .inner
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload);

        for (key, value) in &self.inner.config.headers {
            request = request.header(key, value);
        }

        // For notifications, we don't wait for response body
        let _ = request
            .send()
            .await
            .map_err(|e| ToolInvokeError::Transport {
                server: self.inner.config.name.clone(),
                message: format!("HTTP notification failed: {}", e),
            })?;

        Ok(())
    }

    async fn call_tool(&self, tool: &str, arguments: Value) -> Result<Value, ToolInvokeError> {
        // Ensure connected
        self.connect().await?;

        let params = json!({
            "name": tool,
            "arguments": match arguments {
                Value::Null => Value::Object(Default::default()),
                other => other,
            }
        });

        self.send_request("tools/call", params).await
    }

    async fn instructions(&self) -> Option<String> {
        self.inner.instructions.lock().await.clone()
    }

    async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo> {
        self.inner.tool_cache.lock().await.get(tool).cloned()
    }

    fn server_name(&self) -> &str {
        &self.inner.config.name
    }

    async fn is_connected(&self) -> bool {
        self.inner.connected.load(Ordering::SeqCst)
    }

    async fn disconnect(&self) {
        self.inner.connected.store(false, Ordering::SeqCst);
        self.inner.tool_cache.lock().await.clear();
        *self.inner.instructions.lock().await = None;
    }
}

impl HttpTransport {
    async fn refresh_tools(&self) -> Result<(), ToolInvokeError> {
        let result = self.send_request("tools/list", json!({})).await?;
        self.populate_tool_cache(result).await;
        Ok(())
    }

    async fn populate_tool_cache(&self, result: Value) {
        if let Some(array) = result.get("tools").and_then(Value::as_array) {
            let mut cache = self.inner.tool_cache.lock().await;
            cache.clear();
            for tool in array {
                if let Some(name) = tool.get("name").and_then(Value::as_str) {
                    let description = tool
                        .get("description")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string());
                    let schema = tool.get("inputSchema").cloned();
                    cache.insert(
                        name.to_string(),
                        ServerToolInfo {
                            name: name.to_string(),
                            description,
                            input_schema: schema,
                        },
                    );
                }
            }
            debug!(
                server = %self.inner.config.name,
                tool_count = cache.len(),
                "Refreshed tool cache from HTTP server"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_config() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
        };
        let transport = HttpTransport::new(config);
        assert_eq!(transport.server_name(), "test");
        assert_eq!(transport.rpc_url(), "https://example.com/mcp/rpc");
    }

    #[test]
    fn test_rpc_url_trailing_slash() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://example.com/mcp/".to_string(),
            headers: HashMap::new(),
        };
        let transport = HttpTransport::new(config);
        assert_eq!(transport.rpc_url(), "https://example.com/mcp/rpc");
    }
}
