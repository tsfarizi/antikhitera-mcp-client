//! Server Manager - manages MCP server connections.
//!
//! Handles both STDIO and HTTP transport connections.

use super::error::ToolInvokeError;
use super::interface::{ServerToolInfo, ToolServerInterface};
#[cfg(feature = "native-transport")]
use super::process::McpProcess;
use super::transport::{HttpTransport, HttpTransportConfig, McpTransport, TransportMode};
use crate::config::{ServerConfig, TransportType};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Unified server instance that wraps either STDIO or HTTP transport.
enum ServerInstance {
    #[cfg(feature = "native-transport")]
    Stdio(Arc<McpProcess>),
    Http(Arc<HttpTransport>),
}

impl ServerInstance {
    async fn call_tool(&self, tool: &str, arguments: Value) -> Result<Value, ToolInvokeError> {
        match self {
            #[cfg(feature = "native-transport")]
            ServerInstance::Stdio(process) => process.call_tool(tool, arguments).await,
            ServerInstance::Http(transport) => transport.call_tool(tool, arguments).await,
        }
    }

    async fn instructions(&self) -> Option<String> {
        match self {
            #[cfg(feature = "native-transport")]
            ServerInstance::Stdio(process) => process.instructions().await,
            ServerInstance::Http(transport) => transport.instructions().await,
        }
    }

    async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo> {
        match self {
            #[cfg(feature = "native-transport")]
            ServerInstance::Stdio(process) => process.tool_metadata(tool).await,
            ServerInstance::Http(transport) => transport.tool_metadata(tool).await,
        }
    }
}

pub struct ServerManager {
    configs: HashMap<String, ServerConfig>,
    instances: Mutex<HashMap<String, ServerInstance>>,
}

impl ServerManager {
    pub fn new(configs: Vec<ServerConfig>) -> Self {
        let configs = configs
            .into_iter()
            .map(|cfg| (cfg.name.clone(), cfg))
            .collect();
        Self {
            configs,
            instances: Mutex::new(HashMap::new()),
        }
    }

    async fn ensure_instance(&self, server: &str) -> Result<(), ToolInvokeError> {
        if server.is_empty() {
            return Err(ToolInvokeError::NotConfigured {
                server: server.to_string(),
            });
        }

        // Check if already exists
        {
            let instances = self.instances.lock().expect("server registry lock");
            if instances.contains_key(server) {
                return Ok(());
            }
        }

        // Get config and create instance
        let config =
            self.configs
                .get(server)
                .cloned()
                .ok_or_else(|| ToolInvokeError::NotConfigured {
                    server: server.to_string(),
                })?;

        let instance = match config.transport {
            TransportType::Stdio => {
                #[cfg(feature = "native-transport")]
                {
                let process = Arc::new(McpProcess::new(config));
                process.ensure_running().await?;
                ServerInstance::Stdio(process)
                }
                #[cfg(not(feature = "native-transport"))]
                {
                    return Err(ToolInvokeError::Transport {
                        server: server.to_string(),
                        message: "STDIO transport requires the native-transport feature".to_string(),
                    });
                }
            }
            TransportType::Http => {
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| ToolInvokeError::NotConfigured {
                        server: format!("{}: missing URL for HTTP transport", server),
                    })?;
                let transport_config = HttpTransportConfig {
                    name: config.name.clone(),
                    url,
                    headers: config.headers.clone(),
                    mode: TransportMode::Auto,
                };
                let transport = Arc::new(HttpTransport::new(transport_config));
                transport.connect().await?;
                ServerInstance::Http(transport)
            }
        };

        let mut instances = self.instances.lock().expect("server registry lock");
        instances.insert(server.to_string(), instance);
        Ok(())
    }

    fn get_instance(&self, server: &str) -> Option<ServerInstance> {
        let instances = self.instances.lock().expect("server registry lock");
        match instances.get(server) {
            #[cfg(feature = "native-transport")]
            Some(ServerInstance::Stdio(p)) => Some(ServerInstance::Stdio(p.clone())),
            Some(ServerInstance::Http(t)) => Some(ServerInstance::Http(t.clone())),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl ToolServerInterface for ServerManager {
    async fn invoke_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        self.ensure_instance(server).await?;
        let instance = self
            .get_instance(server)
            .ok_or_else(|| ToolInvokeError::NotConfigured {
                server: server.to_string(),
            })?;
        instance.call_tool(tool, arguments).await
    }

    async fn server_instructions(&self, server: &str) -> Option<String> {
        match self.ensure_instance(server).await {
            Ok(()) => {
                if let Some(instance) = self.get_instance(server) {
                    instance.instructions().await
                } else {
                    None
                }
            }
            Err(err) => {
                warn!(server, %err, "Failed to fetch server instructions");
                None
            }
        }
    }

    async fn tool_metadata(&self, server: &str, tool: &str) -> Option<ServerToolInfo> {
        match self.ensure_instance(server).await {
            Ok(()) => {
                if let Some(instance) = self.get_instance(server) {
                    instance.tool_metadata(tool).await
                } else {
                    None
                }
            }
            Err(err) => {
                warn!(server, tool, %err, "Failed to fetch tool metadata");
                None
            }
        }
    }
}
