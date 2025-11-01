use super::error::ToolInvokeError;
use super::interface::{ServerToolInfo, ToolServerInterface};
use super::process::McpProcess;
use crate::config::ServerConfig;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::warn;

pub struct ServerManager {
    configs: HashMap<String, ServerConfig>,
    instances: Mutex<HashMap<String, Arc<McpProcess>>>,
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

    async fn ensure_process(&self, server: &str) -> Result<Arc<McpProcess>, ToolInvokeError> {
        if server.is_empty() {
            return Err(ToolInvokeError::NotConfigured {
                server: server.to_string(),
            });
        }

        let process = {
            let mut instances = self.instances.lock().expect("server registry lock");
            if let Some(existing) = instances.get(server) {
                existing.clone()
            } else {
                let config = self.configs.get(server).cloned().ok_or_else(|| {
                    ToolInvokeError::NotConfigured {
                        server: server.to_string(),
                    }
                })?;
                let process = Arc::new(McpProcess::new(config));
                instances.insert(server.to_string(), process.clone());
                process
            }
        };

        process.ensure_running().await?;
        Ok(process)
    }
}

#[async_trait]
impl ToolServerInterface for ServerManager {
    async fn invoke_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        let process = self.ensure_process(server).await?;
        process.call_tool(tool, arguments).await
    }

    async fn server_instructions(&self, server: &str) -> Option<String> {
        match self.ensure_process(server).await {
            Ok(process) => process.instructions().await,
            Err(err) => {
                warn!(server, %err, "Failed to fetch server instructions");
                None
            }
        }
    }

    async fn tool_metadata(&self, server: &str, tool: &str) -> Option<ServerToolInfo> {
        match self.ensure_process(server).await {
            Ok(process) => process.tool_metadata(tool).await,
            Err(err) => {
                warn!(server, tool, %err, "Failed to fetch tool metadata");
                None
            }
        }
    }
}
