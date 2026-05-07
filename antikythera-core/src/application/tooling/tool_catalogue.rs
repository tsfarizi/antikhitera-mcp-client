use super::error::ToolInvokeError;
use super::interface::{ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution, ToolIcon};
use super::process::{McpProcess, McpProcessInner};
use super::transport::{HttpTransport, HttpTransportConfig, McpTransport, TransportMode};
use serde_json::{Value, json};

use crate::config::ServerConfig;
use crate::infrastructure::mcp::validate_tool_name;
use crate::logging::TransportLogger;

impl McpProcessInner {
    pub(crate) async fn refresh_tools(&self) -> Result<(), ToolInvokeError> {
        let mut cursor: Option<String> = None;
        loop {
            let params = if let Some(ref c) = cursor {
                json!({ "cursor": c })
            } else {
                json!({})
            };
            let result = self.send_request("tools/list", params).await?;
            cursor = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            self.populate_tool_cache(result, cursor.is_none()).await;
            if cursor.is_none() {
                break;
            }
        }
        Ok(())
    }

    async fn populate_tool_cache(&self, result: Value, clear_first: bool) {
        if let Some(array) = result.get("tools").and_then(Value::as_array) {
            let mut cache = self.tool_cache.lock().await;
            if clear_first {
                cache.clear();
            }
            for tool in array {
                if let Some(name) = tool.get("name").and_then(Value::as_str) {
                    let name = name.to_string();
                    if validate_tool_name(&name).is_err() {
                        TransportLogger::new(&self.server.name).warn(format!(
                            "Skipping tool with invalid name | server={} tool={}",
                            self.server.name, name
                        ));
                        continue;
                    }
                    let title = tool
                        .get("title")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string());
                    let description = tool
                        .get("description")
                        .and_then(Value::as_str)
                        .map(|text| text.to_string());
                    let icons = tool.get("icons").and_then(Value::as_array).map(|arr| {
                        arr.iter()
                            .filter_map(|icon| {
                                Some(ToolIcon {
                                    src: icon.get("src")?.as_str()?.to_string(),
                                    mime_type: icon
                                        .get("mimeType")
                                        .and_then(Value::as_str)
                                        .map(|s| s.to_string()),
                                    sizes: icon.get("sizes").and_then(Value::as_array).map(|sz| {
                                        sz.iter()
                                            .filter_map(|s| s.as_str().map(|v| v.to_string()))
                                            .collect()
                                    }),
                                })
                            })
                            .collect()
                    });
                    let input_schema = tool.get("inputSchema").cloned();
                    let output_schema = tool.get("outputSchema").cloned();
                    let annotations =
                        tool.get("annotations")
                            .and_then(Value::as_object)
                            .map(|ann| ToolAnnotations {
                                audience: ann.get("audience").and_then(Value::as_array).map(|a| {
                                    a.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                }),
                                priority: ann.get("priority").and_then(Value::as_f64),
                                last_modified: ann
                                    .get("lastModified")
                                    .and_then(Value::as_str)
                                    .map(|s| s.to_string()),
                            });
                    let execution =
                        tool.get("execution")
                            .and_then(Value::as_object)
                            .map(|exe| ToolExecution {
                                task_support: exe
                                    .get("taskSupport")
                                    .and_then(Value::as_str)
                                    .and_then(|v| match v {
                                        "forbidden" => Some(TaskSupport::Forbidden),
                                        "optional" => Some(TaskSupport::Optional),
                                        "required" => Some(TaskSupport::Required),
                                        _ => None,
                                    }),
                            });
                    cache.insert(
                        name.clone(),
                        ServerToolInfo {
                            name,
                            title,
                            description,
                            icons,
                            input_schema,
                            output_schema,
                            annotations,
                            execution,
                        },
                    );
                }
            }
        }
    }
}

/// Spawn an MCP server process and list its available tools.
/// Returns a list of (tool_name, description) pairs.
pub async fn spawn_and_list_tools(
    config: &ServerConfig,
) -> Result<Vec<(String, String)>, ToolInvokeError> {
    if config.is_http() {
        let url = config
            .url
            .clone()
            .ok_or_else(|| ToolInvokeError::NotConfigured {
                server: format!("{}: missing URL for HTTP transport", config.name),
            })?;
        let transport_config = HttpTransportConfig {
            name: config.name.clone(),
            url,
            headers: config.headers.clone(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };
        let transport = HttpTransport::new(transport_config);
        transport.connect().await?;
        let tools = transport.list_tools().await;
        Ok(tools
            .into_iter()
            .map(|info| (info.name, info.description.unwrap_or_default()))
            .collect())
    } else {
        let process = McpProcess::new(config.clone());
        process.ensure_running().await?;
        let cache = process.inner.tool_cache.lock().await;
        let tools: Vec<(String, String)> = cache
            .values()
            .map(|info| {
                (
                    info.name.clone(),
                    info.description.clone().unwrap_or_default(),
                )
            })
            .collect();

        Ok(tools)
    }
}
