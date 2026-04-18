//! HTTP Transport for MCP servers.
//!
//! Main implementation that coordinates SSE, RPC, and tool caching modules.

mod rpc;
mod sse;
mod tools;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

use super::McpTransport;
use super::config::{HttpTransportConfig, TransportMode};
use crate::application::tooling::error::ToolInvokeError;
use crate::application::tooling::interface::ServerToolInfo;

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
    active_mode: AsyncMutex<Option<TransportMode>>,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(config: HttpTransportConfig) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        #[cfg(target_arch = "wasm32")]
        let client = Client::builder()
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
                active_mode: AsyncMutex::new(None),
            }),
        }
    }

    /// Get the server name.
    pub fn get_name(&self) -> &str {
        &self.inner.config.name
    }

    /// Start SSE listener in background.
    fn start_sse_listener(&self) {
        // Clone the Arc to the inner, then we need to clone the session_endpoint field
        let inner = self.inner.clone();
        let session_endpoint = Arc::new(AsyncMutex::new(None));
        let session_endpoint_clone = session_endpoint.clone();

        // Start SSE listener
        sse::start_sse_listener(
            inner.client.clone(),
            inner.config.name.clone(),
            inner.config.url.clone(),
            inner.config.headers.clone(),
            session_endpoint_clone,
        );

        // Sync the session endpoint back to inner in a separate task
        let inner_for_sync = self.inner.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                if let Some(endpoint) = session_endpoint.lock().await.as_ref() {
                    *inner_for_sync.session_endpoint.lock().await = Some(endpoint.clone());
                    break;
                }
            }
        });
    }

    /// Resolve session endpoint URL.
    async fn resolve_endpoint(&self) -> Result<String, ToolInvokeError> {
        sse::resolve_endpoint(
            &self.inner.config.name,
            &self.inner.config.url,
            &self.inner.session_endpoint,
        )
        .await
    }

    /// Refresh tools from server.
    async fn refresh_tools(&self) -> Result<(), ToolInvokeError> {
        let result = self.send_request("tools/list", json!({})).await?;
        tools::populate_tool_cache(&self.inner.config.name, &self.inner.tool_cache, result).await;
        Ok(())
    }
}

#[async_trait(?Send)]
impl McpTransport for HttpTransport {
    async fn connect(&self) -> Result<(), ToolInvokeError> {
        if self.inner.connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        let configured_mode = self.inner.config.mode;

        info!(
            server = %self.inner.config.name,
            url = %self.inner.config.url,
            mode = ?configured_mode,
            "Connecting to HTTP MCP server"
        );

        // Determine transport mode
        let detected_mode = match configured_mode {
            TransportMode::Stateful => {
                #[cfg(target_arch = "wasm32")]
                {
                    return Err(ToolInvokeError::Transport {
                        server: self.inner.config.name.clone(),
                        message: "Stateful SSE mode is not supported on wasm32 targets".to_string(),
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                self.start_sse_listener();
                match self.resolve_endpoint().await {
                    Ok(_) => TransportMode::Stateful,
                    Err(e) => {
                        tracing::warn!(server = %self.inner.config.name, %e, "SSE connection failed");
                        return Err(e);
                    }
                }
                }
            }
            TransportMode::Stateless => {
                info!(server = %self.inner.config.name, "Using stateless mode (direct HTTP POST)");
                *self.inner.session_endpoint.lock().await = Some(self.inner.config.url.clone());
                TransportMode::Stateless
            }
            TransportMode::Auto => {
                #[cfg(target_arch = "wasm32")]
                {
                    info!(server = %self.inner.config.name, "Using stateless mode on wasm32 target");
                    *self.inner.session_endpoint.lock().await = Some(self.inner.config.url.clone());
                    TransportMode::Stateless
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                info!(server = %self.inner.config.name, "Auto-detecting transport mode...");
                self.start_sse_listener();

                match self.resolve_endpoint().await {
                    Ok(_) => {
                        info!(server = %self.inner.config.name, "Detected stateful mode (SSE endpoint received)");
                        TransportMode::Stateful
                    }
                    Err(_) => {
                        info!(server = %self.inner.config.name, "SSE timeout - falling back to stateless mode");
                        *self.inner.session_endpoint.lock().await =
                            Some(self.inner.config.url.clone());
                        TransportMode::Stateless
                    }
                }
                }
            }
        };

        *self.inner.active_mode.lock().await = Some(detected_mode);

        // Initialize connection
        let params = json!({
            "protocolVersion": rpc::PROTOCOL_VERSION,
            "clientInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
                "title": "CBT MCP Client"
            },
            "capabilities": {}
        });

        let result = self.send_request("initialize", params).await?;

        if let Some(text) = result.get("instructions").and_then(Value::as_str) {
            *self.inner.instructions.lock().await = Some(text.to_string());
        }

        self.send_notification("notifications/initialized", json!({}))
            .await?;
        self.refresh_tools().await?;

        self.inner.connected.store(true, Ordering::SeqCst);

        let tool_count = self.inner.tool_cache.lock().await.len();
        info!(
            server = %self.inner.config.name,
            tool_count = tool_count,
            mode = ?detected_mode,
            "Successfully connected to HTTP MCP server"
        );

        Ok(())
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value, ToolInvokeError> {
        let url = self.resolve_endpoint().await?;
        rpc::send_request(
            &self.inner.client,
            &self.inner.config.name,
            &url,
            method,
            params,
            &self.inner.config.headers,
            &self.inner.id_counter,
        )
        .await
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolInvokeError> {
        let url = self.resolve_endpoint().await?;
        rpc::send_notification(
            &self.inner.client,
            &self.inner.config.name,
            &url,
            method,
            params,
            &self.inner.config.headers,
        )
        .await
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

    async fn list_tools(&self) -> Vec<ServerToolInfo> {
        self.inner
            .tool_cache
            .lock()
            .await
            .values()
            .cloned()
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_transport_list_tools() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
        };
        let transport = HttpTransport::new(config);

        // Manually populate cache
        {
            let mut cache = transport.inner.tool_cache.lock().await;
            cache.insert(
                "test_tool".to_string(),
                ServerToolInfo {
                    name: "test_tool".to_string(),
                    description: Some("test description".to_string()),
                    input_schema: None,
                },
            );
        }

        let tools = transport.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
        assert_eq!(tools[0].description, Some("test description".to_string()));
    }

    #[tokio::test]
    async fn test_http_transport_disconnect() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
        };
        let transport = HttpTransport::new(config);
        transport.inner.connected.store(true, Ordering::SeqCst);

        // Populate cache
        {
            let mut cache = transport.inner.tool_cache.lock().await;
            cache.insert(
                "test_tool".to_string(),
                ServerToolInfo {
                    name: "test_tool".to_string(),
                    description: None,
                    input_schema: None,
                },
            );
        }

        transport.disconnect().await;

        assert!(!transport.is_connected().await);
        assert!(transport.list_tools().await.is_empty());
    }
}
