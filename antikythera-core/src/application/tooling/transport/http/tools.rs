//! Tool cache management for HTTP transport.
//!
//! Handles refreshing and populating the tool metadata cache.

use crate::logging::TransportLogger;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::Mutex as AsyncMutex;

use crate::application::tooling::interface::{
    ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution, ToolIcon,
};
use crate::infrastructure::mcp::validate_tool_name;

/// Populate the tool cache from a tools/list response.
pub async fn populate_tool_cache(
    server_name: &str,
    tool_cache: &AsyncMutex<HashMap<String, ServerToolInfo>>,
    result: Value,
    clear_first: bool,
) {
    if let Some(array) = result.get("tools").and_then(Value::as_array) {
        let mut cache = tool_cache.lock().await;
        if clear_first {
            cache.clear();
        }
        for tool in array {
            if let Some(name) = tool.get("name").and_then(Value::as_str) {
                let name = name.to_string();
                if validate_tool_name(&name).is_err() {
                    TransportLogger::new(server_name).warn(format!(
                        "Skipping tool with invalid name | server={} tool={}",
                        server_name, name
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
                    .map(|s| s.to_string());
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
                let annotations = tool
                    .get("annotations")
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
                            task_support: exe.get("taskSupport").and_then(Value::as_str).and_then(
                                |v| match v {
                                    "forbidden" => Some(TaskSupport::Forbidden),
                                    "optional" => Some(TaskSupport::Optional),
                                    "required" => Some(TaskSupport::Required),
                                    _ => None,
                                },
                            ),
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
        TransportLogger::new(server_name).debug(format!(
            "Refreshed tool cache from HTTP server | server={} tool_count={}",
            server_name,
            cache.len()
        ));
    }
}
