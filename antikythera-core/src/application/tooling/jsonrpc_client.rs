use super::error::ToolInvokeError;
use super::process::McpProcessInner;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::oneshot;

use crate::logging::TransportLogger;

impl McpProcessInner {
    pub(crate) async fn send_request(
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

    pub(crate) async fn send_notification(
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

    async fn send_response(&self, id: Value, result: Value) -> Result<(), ToolInvokeError> {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "result": result
        });
        if let Value::Object(ref mut map) = payload {
            map.insert("id".to_string(), id);
        }
        self.write_message(&payload).await
    }

    async fn send_error(&self, id: Value, error: Value) -> Result<(), ToolInvokeError> {
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
        let encoded =
            serde_json::to_string(message).map_err(|source| ToolInvokeError::InvalidJson {
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

    pub(crate) async fn reader_loop(self: Arc<Self>, stdout: ChildStdout) {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(item) = lines.next_line().await {
            match item {
                Some(raw) => {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed.starts_with('\u{1b}') {
                        TransportLogger::new(&self.server.name).debug(format!(
                            "skipping non-JSON ANSI log line from MCP server | server={} line={}",
                            self.server.name, trimmed
                        ));
                        continue;
                    }
                    match serde_json::from_str::<Value>(&raw) {
                        Ok(value) => {
                            if let Err(err) = self.process_inbound_message(value).await {
                                TransportLogger::new(&self.server.name).warn(format!(
                                    "failed to process message from MCP server | server={} error={}",
                                    self.server.name, err
                                ));
                            }
                        }
                        Err(source) => {
                            TransportLogger::new(&self.server.name).warn(format!(
                                "received invalid JSON from MCP server | server={} line={} source={}",
                                self.server.name, raw, source
                            ));
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

    async fn handle_response(&self, id: Value, value: Value) -> Result<(), ToolInvokeError> {
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
                let error = value.get("error").and_then(Value::as_object).map(|err| {
                    (
                        err.get("code").and_then(Value::as_i64).unwrap_or(-32000),
                        err.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown error")
                            .to_string(),
                    )
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
            TransportLogger::new(&self.server.name).debug(format!(
                "received response for unknown request | server={} response_id={}",
                self.server.name, key
            ));
        }
        Ok(())
    }

    async fn handle_server_request(&self, id: Value, value: Value) -> Result<(), ToolInvokeError> {
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match method {
            "ping" => {
                self.send_response(id, json!({ "ok": true })).await?;
            }
            "elicitation/create" => {
                let params = value.get("params").cloned().unwrap_or(Value::Null);
                let response = self.build_elicitation_ack(params);
                self.send_response(id, response).await?;
            }
            other => {
                TransportLogger::new(&self.server.name).warn(format!(
                    "server sent unsupported request | server={} method={}",
                    self.server.name, other
                ));
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
            TransportLogger::new(&self.server.name).debug(format!(
                "received notification from server | server={} method={}",
                self.server.name, method
            ));
            if method == "notifications/tools/list_changed"
                && let Err(err) = self.refresh_tools().await
            {
                TransportLogger::new(&self.server.name).warn(format!(
                    "failed to refresh tool catalogue | server={} error={}",
                    self.server.name, err
                ));
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
}
