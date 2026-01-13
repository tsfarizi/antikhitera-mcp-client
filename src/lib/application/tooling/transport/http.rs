//! HTTP Transport for MCP servers.
//!
//! Implements JSON-RPC 2.0 over HTTP/SSE for MCP communication.

use async_trait::async_trait;
use reqwest::Client;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex as AsyncMutex;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

use super::McpTransport;
use crate::application::tooling::error::ToolInvokeError;
use crate::application::tooling::interface::ServerToolInfo;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// HTTP Transport configuration.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Server name identifier
    pub name: String,
    /// Base URL for the MCP server (SSE endpoint)
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
    session_endpoint: AsyncMutex<Option<String>>,
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
                session_endpoint: AsyncMutex::new(None),
            }),
        }
    }

    /// Get the server name.
    pub fn get_name(&self) -> &str {
        &self.inner.config.name
    }

    /// Start listening to SSE events in bg
    fn start_sse_listener(&self) {
        let client = self.inner.client.clone();
        let name = self.inner.config.name.clone();
        let url = self.inner.config.url.clone();
        let inner = self.inner.clone();

        tokio::spawn(async move {
            debug!(server = %name, %url, "Starting SSE listener");

            let mut request = client.get(&url);

            // Add custom headers
            for (key, value) in &inner.config.headers {
                if key.eq_ignore_ascii_case("Authorization") {
                    if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("Bearer") {
                        continue;
                    }
                }
                request = request.header(key, value);
            }

            let mut es = request.eventsource().unwrap();

            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => {
                        info!(server = %name, "SSE connection opened");
                    }
                    Ok(Event::Message(message)) => {
                        debug!(server = %name, event = %message.event, "Received SSE event");
                        if message.event == "endpoint" {
                            let endpoint = message.data.trim().to_string();
                            info!(server = %name, %endpoint, "Received session endpoint");
                            *inner.session_endpoint.lock().await = Some(endpoint);
                        }
                    }
                    Err(err) => {
                        warn!(server = %name, %err, "Error in SSE stream");
                        // Decide if we should exit or retry. EventSource handles reconnects implicitly usually.
                    }
                }
            }
            warn!(server = %name, "SSE stream ended");
        });
    }

    async fn resolve_endpoint(&self) -> Result<String, ToolInvokeError> {
        // Wait for session endpoint to be available (with timeout)
        let start = tokio::time::Instant::now();
        loop {
            if let Some(endpoint) = self.inner.session_endpoint.lock().await.as_ref() {
                // Handle relative URLs
                if endpoint.starts_with("http") {
                    return Ok(endpoint.clone());
                } else {
                    // If endpoint starts with /, join carefully
                    // NOTE: This assumes the config URL is the base for the relative endpoint.
                    // If config URL is /sse, and endpoint is /sse, we might just want to use config URL base?
                    // Standard practice: config URL is connection URL.
                    // If endpoint is relative, it's relative to connection URL? Or host?
                    // Let's assume relative to host of the connection URL.
                    let url = reqwest::Url::parse(&self.inner.config.url).map_err(|e| {
                        ToolInvokeError::Transport {
                            server: self.inner.config.name.clone(),
                            message: format!("Invalid base URL: {}", e),
                        }
                    })?;

                    let joined = url.join(endpoint).map_err(|e| ToolInvokeError::Transport {
                        server: self.inner.config.name.clone(),
                        message: format!("Failed to join endpoint: {}", e),
                    })?;
                    return Ok(joined.to_string());
                }
            }

            if start.elapsed() > std::time::Duration::from_secs(5) {
                return Err(ToolInvokeError::Transport {
                    server: self.inner.config.name.clone(),
                    message: "Timed out waiting for session endpoint".to_string(),
                });
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
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

        // Start SSE listener
        self.start_sse_listener();

        // Wait for endpoint resolution
        let _endpoint = self.resolve_endpoint().await?;

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

        let url = self.resolve_endpoint().await?;

        debug!(
            server = %self.inner.config.name,
            method = method,
            url = %url,
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
            if key.eq_ignore_ascii_case("Authorization") {
                if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("Bearer") {
                    continue;
                }
            }
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

        let url = self.resolve_endpoint().await?;

        let mut request = self
            .inner
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload);

        for (key, value) in &self.inner.config.headers {
            if key.eq_ignore_ascii_case("Authorization") {
                if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("Bearer") {
                    continue;
                }
            }
            request = request.header(key, value);
        }

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
        *self.inner.session_endpoint.lock().await = None;
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
        assert_eq!(transport.get_name(), "test");
    }
}
