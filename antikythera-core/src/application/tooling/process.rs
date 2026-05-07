use super::error::ToolInvokeError;
use super::interface::{PROTOCOL_VERSION, ServerToolInfo};
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::io::BufWriter;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex as AsyncMutex, oneshot};

use crate::config::ServerConfig;
use crate::logging::TransportLogger;

#[derive(Clone)]
pub struct McpProcess {
    pub(super) inner: Arc<McpProcessInner>,
}

pub(super) struct McpProcessInner {
    pub(super) server: ServerConfig,
    state: AsyncMutex<Option<RunningState>>,
    pub(super) writer: AsyncMutex<Option<BufWriter<ChildStdin>>>,
    pub(super) pending:
        AsyncMutex<HashMap<String, oneshot::Sender<Result<Value, ToolInvokeError>>>>,
    pub(super) id_counter: AtomicU64,
    instructions: AsyncMutex<Option<String>>,
    pub(super) tool_cache: AsyncMutex<HashMap<String, ServerToolInfo>>,
}

struct RunningState {
    child: Child,
}

impl McpProcess {
    pub fn new(server: ServerConfig) -> Self {
        Self {
            inner: Arc::new(McpProcessInner {
                server,
                state: AsyncMutex::new(None),
                writer: AsyncMutex::new(None),
                pending: AsyncMutex::new(HashMap::new()),
                id_counter: AtomicU64::new(1),
                instructions: AsyncMutex::new(None),
                tool_cache: AsyncMutex::new(HashMap::new()),
            }),
        }
    }

    pub(super) async fn ensure_running(&self) -> Result<(), ToolInvokeError> {
        self.inner.ensure_running().await
    }

    pub(super) async fn call_tool(
        &self,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        self.ensure_running().await?;
        self.inner.call_tool(tool, arguments).await
    }

    pub(super) async fn instructions(&self) -> Option<String> {
        self.inner.instructions.lock().await.clone()
    }

    pub(super) async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo> {
        self.inner.tool_cache.lock().await.get(tool).cloned()
    }
}

impl McpProcessInner {
    async fn ensure_running(self: &Arc<Self>) -> Result<(), ToolInvokeError> {
        {
            let state = self.state.lock().await;
            if state.is_some() {
                return Ok(());
            }
        }

        let command_path =
            self.server
                .command
                .as_ref()
                .ok_or_else(|| ToolInvokeError::NotConfigured {
                    server: format!("{}: no command path configured", self.server.name),
                })?;

        let mut command = Command::new(command_path);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(dir) = &self.server.workdir {
            command.current_dir(dir);
        }
        if !self.server.args.is_empty() {
            command.args(&self.server.args);
        }
        for (key, value) in &self.server.env {
            command.env(key, value);
        }

        let mut child = command.spawn().map_err(|source| ToolInvokeError::Spawn {
            server: self.server.name.clone(),
            source,
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| self.transport_error("failed to capture server stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| self.transport_error("failed to capture server stdout"))?;

        {
            let mut writer = self.writer.lock().await;
            *writer = Some(BufWriter::new(stdin));
        }

        {
            let mut state = self.state.lock().await;
            *state = Some(RunningState { child });
        }

        let reader_self = Arc::clone(self);
        tokio::spawn(async move {
            reader_self.reader_loop(stdout).await;
        });

        match self.initialize_sequence().await {
            Ok(_) => Ok(()),
            Err(err) => {
                self.reset().await;
                Err(err)
            }
        }
    }

    async fn initialize_sequence(self: &Arc<Self>) -> Result<(), ToolInvokeError> {
        let params = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
                "title": "CBT MCP Client"
            },
            "capabilities": {
                "tools": {
                    "listChanged": true
                }
            }
        });
        let init_result = self.send_request("initialize", params).await?;
        if let Some(text) = init_result.get("instructions").and_then(Value::as_str) {
            let mut instructions = self.instructions.lock().await;
            *instructions = Some(text.to_string());
        }
        self.send_notification("notifications/initialized", json!({}))
            .await?;

        self.refresh_tools().await?;
        Ok(())
    }

    pub(super) async fn call_tool(
        &self,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        let params = json!({
            "name": tool,
            "arguments": match arguments {
                Value::Null => Value::Object(Default::default()),
                other => other,
            }
        });
        let response = self.send_request("tools/call", params).await?;
        Ok(response)
    }

    pub(crate) fn build_elicitation_ack(&self, params: Value) -> Value {
        let parsed: ElicitationCreateParams = serde_json::from_value(params).unwrap_or_default();

        let mut content = JsonMap::new();
        if let Some(message) = parsed.message {
            let trimmed = message.trim();
            if !trimmed.is_empty() {
                content.insert("message".to_string(), Value::String(trimmed.to_string()));
            }
        }

        let mut response = JsonMap::new();
        response.insert("action".to_string(), Value::String("accept".to_string()));
        response.insert("content".to_string(), Value::Object(content));
        Value::Object(response)
    }

    pub(super) async fn reset(&self) {
        {
            let mut writer = self.writer.lock().await;
            *writer = None;
        }

        let mut state = self.state.lock().await;
        if let Some(mut running) = state.take() {
            if let Err(err) = running.child.kill().await {
                TransportLogger::new(&self.server.name).debug(format!(
                    "failed to kill MCP server process (may have already exited) | server={} error={}",
                    self.server.name, err
                ));
            }
            let _ = running.child.wait().await;
        }
        drop(state);

        self.fail_all_pending().await;
        self.tool_cache.lock().await.clear();
        self.instructions.lock().await.take();
    }

    async fn fail_all_pending(&self) {
        let mut pending = self.pending.lock().await;
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(ToolInvokeError::Terminated {
                server: self.server.name.clone(),
            }));
        }
    }

    pub(super) fn transport_error(&self, message: impl Into<String>) -> ToolInvokeError {
        ToolInvokeError::Transport {
            server: self.server.name.clone(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ElicitationCreateParams {
    #[serde(default)]
    message: Option<String>,
    #[serde(rename = "requestedSchema", default)]
    _requested_schema: Value,
}
