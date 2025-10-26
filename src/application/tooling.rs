use crate::config::ServerConfig;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tracing::{debug, error, warn};

const PROTOCOL_VERSION: &str = "2025-06-18";

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

    async fn tool_metadata(
        &self,
        server: &str,
        tool: &str,
    ) -> Option<ServerToolInfo>;
}

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

    async fn ensure_process(
        &self,
        server: &str,
    ) -> Result<Arc<McpProcess>, ToolInvokeError> {
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
                let config = self
                    .configs
                    .get(server)
                    .cloned()
                    .ok_or_else(|| ToolInvokeError::NotConfigured {
                        server: server.to_string(),
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

    async fn tool_metadata(
        &self,
        server: &str,
        tool: &str,
    ) -> Option<ServerToolInfo> {
        match self.ensure_process(server).await {
            Ok(process) => process.tool_metadata(tool).await,
            Err(err) => {
                warn!(server, tool, %err, "Failed to fetch tool metadata");
                None
            }
        }
    }
}

#[derive(Clone)]
pub struct McpProcess {
    inner: Arc<McpProcessInner>,
}

struct McpProcessInner {
    server: ServerConfig,
    state: AsyncMutex<Option<RunningState>>,
    writer: AsyncMutex<Option<BufWriter<ChildStdin>>>,
    pending: AsyncMutex<HashMap<String, oneshot::Sender<Result<Value, ToolInvokeError>>>>,
    id_counter: AtomicU64,
    instructions: AsyncMutex<Option<String>>,
    tool_cache: AsyncMutex<HashMap<String, ServerToolInfo>>,
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

    async fn ensure_running(&self) -> Result<(), ToolInvokeError> {
        self.inner.ensure_running().await
    }

    async fn call_tool(
        &self,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        self.ensure_running().await?;
        self.inner
            .call_tool(tool, arguments)
            .await
    }

    async fn instructions(&self) -> Option<String> {
        self.inner.instructions.lock().await.clone()
    }

    async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo> {
        self.inner
            .tool_cache
            .lock()
            .await
            .get(tool)
            .cloned()
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

        let mut command = Command::new(&self.server.command);
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

        let mut child = command
            .spawn()
            .map_err(|source| ToolInvokeError::Spawn {
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
            "capabilities": {}
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

    async fn call_tool(
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

    async fn refresh_tools(&self) -> Result<(), ToolInvokeError> {
        let result = self.send_request("tools/list", json!({})).await?;
        self.populate_tool_cache(result).await;
        Ok(())
    }

    async fn reader_loop(self: Arc<Self>, stdout: ChildStdout) {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(item) = lines.next_line().await {
            match item {
                Some(raw) => {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<Value>(&raw) {
                        Ok(value) => {
                            if let Err(err) = self.process_inbound_message(value).await {
                                warn!(
                                    server = %self.server.name,
                                    %err,
                                    "failed to process message from MCP server"
                                );
                            }
                        }
                        Err(source) => {
                            warn!(
                                server = %self.server.name,
                                line = raw,
                                %source,
                                "received invalid JSON from MCP server"
                            );
                        }
                    }
                }
                None => break,
            }
        }

        self.reset().await;
    }

    async fn process_inbound_message(&self, value: Value) -> Result<(), ToolInvokeError> {
        if let Some(id) = value.get("id").cloned() {
            if value.get("method").is_some() {
                self.handle_server_request(id, value).await
            } else {
                self.handle_response(id, value).await
            }
        } else if value.get("method").is_some() {
            self.handle_notification(value).await;
            Ok(())
        } else {
            Ok(())
        }
    }

    async fn handle_response(
        &self,
        id: Value,
        value: Value,
    ) -> Result<(), ToolInvokeError> {
        let key = match self.response_key(&id) {
            Some(key) => key,
            None => return Ok(()),
        };

        let responder = {
            let mut pending = self.pending.lock().await;
            pending.remove(&key)
        };

        if let Some(sender) = responder {
            if value.get("error").is_some() {
                let error = value
                    .get("error")
                    .and_then(Value::as_object)
                    .and_then(|err| {
                        Some((
                            err.get("code").and_then(Value::as_i64).unwrap_or(-32000),
                            err.get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown error")
                                .to_string(),
                        ))
                    });
                let rpc_error = match error {
                    Some((code, message)) => ToolInvokeError::Rpc {
                        server: self.server.name.clone(),
                        code,
                        message,
                    },
                    None => self.transport_error("missing error payload in response"),
                };
                let _ = sender.send(Err(rpc_error));
            } else {
                let _ = sender.send(Ok(value));
            }
        } else {
            debug!(
                server = %self.server.name,
                response_id = key,
                "received response for unknown request"
            );
        }
        Ok(())
    }

    async fn handle_server_request(
        &self,
        id: Value,
        value: Value,
    ) -> Result<(), ToolInvokeError> {
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match method {
            "ping" => {
                self.send_response(id, json!({ "ok": true })).await?;
            }
            other => {
                warn!(
                    server = %self.server.name,
                    method = other,
                    "server sent unsupported request"
                );
                let error = json!({
                    "code": -32601,
                    "message": format!("client does not implement method '{other}'"),
                });
                self.send_error(id, error).await?;
            }
        }
        Ok(())
    }

    async fn handle_notification(&self, value: Value) {
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            debug!(
                server = %self.server.name,
                method,
                "received notification from server"
            );
            if method == "notifications/tools/list_changed" {
                if let Err(err) = self.refresh_tools().await {
                    warn!(
                        server = %self.server.name,
                        %err,
                        "failed to refresh tool catalogue"
                    );
                }
            }
        }
    }

    async fn send_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, ToolInvokeError> {
        let id = self.next_id();
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id.clone(), tx);
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_message(&payload).await?;

        match rx.await {
            Ok(Ok(value)) => {
                let result = value.get("result").cloned().unwrap_or(Value::Null);
                Ok(result)
            }
            Ok(Err(err)) => Err(err),
            Err(_) => Err(ToolInvokeError::Cancelled {
                server: self.server.name.clone(),
            }),
        }
    }

    async fn send_notification(
        &self,
        method: &str,
        params: Value,
    ) -> Result<(), ToolInvokeError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.write_message(&payload).await
    }

    async fn send_response(
        &self,
        id: Value,
        result: Value,
    ) -> Result<(), ToolInvokeError> {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "result": result
        });
        if let Value::Object(ref mut map) = payload {
            map.insert("id".to_string(), id);
        }
        self.write_message(&payload).await
    }

    async fn send_error(
        &self,
        id: Value,
        error: Value,
    ) -> Result<(), ToolInvokeError> {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "error": error
        });
        if let Value::Object(ref mut map) = payload {
            map.insert("id".to_string(), id);
        }
        self.write_message(&payload).await
    }

    async fn write_message(&self, message: &Value) -> Result<(), ToolInvokeError> {
        let encoded = serde_json::to_string(message).map_err(|source| ToolInvokeError::InvalidJson {
            server: self.server.name.clone(),
            source,
        })?;

        let mut writer = self.writer.lock().await;
        let stream = writer
            .as_mut()
            .ok_or_else(|| self.transport_error("writer not initialised"))?;
        stream
            .write_all(encoded.as_bytes())
            .await
            .map_err(|source| ToolInvokeError::Transport {
                server: self.server.name.clone(),
                message: source.to_string(),
            })?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|source| ToolInvokeError::Transport {
                server: self.server.name.clone(),
                message: source.to_string(),
            })?;
        stream
            .flush()
            .await
            .map_err(|source| ToolInvokeError::Transport {
                server: self.server.name.clone(),
                message: source.to_string(),
            })?;
        Ok(())
    }

    async fn reset(&self) {
        {
            let mut writer = self.writer.lock().await;
            *writer = None;
        }

        let mut state = self.state.lock().await;
        if let Some(mut running) = state.take() {
            if let Err(err) = running.child.kill().await {
                debug!(
                    server = %self.server.name,
                    %err,
                    "failed to kill MCP server process (may have already exited)"
                );
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

    async fn populate_tool_cache(&self, result: Value) {
        if let Some(array) = result.get("tools").and_then(Value::as_array) {
            let mut cache = self.tool_cache.lock().await;
            cache.clear();
            for tool in array {
                if let Some(name) = tool.get("name").and_then(Value::as_str) {
                    let description = tool
                        .get("description")
                        .and_then(Value::as_str)
                        .map(|text| text.to_string());
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
        }
    }

    fn next_id(&self) -> String {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        format!("req-{id}")
    }

    fn response_key(&self, id: &Value) -> Option<String> {
        match id {
            Value::String(value) => Some(value.clone()),
            Value::Number(num) => Some(num.to_string()),
            _ => None,
        }
    }

    fn transport_error(&self, message: impl Into<String>) -> ToolInvokeError {
        ToolInvokeError::Transport {
            server: self.server.name.clone(),
            message: message.into(),
        }
    }
}
