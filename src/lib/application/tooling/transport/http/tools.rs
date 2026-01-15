//! Tool cache management for HTTP transport.
//!
//! Handles refreshing and populating the tool metadata cache.

use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::Mutex as AsyncMutex;
use tracing::debug;

use crate::application::tooling::interface::ServerToolInfo;

/// Populate the tool cache from a tools/list response.
pub async fn populate_tool_cache(
    server_name: &str,
    tool_cache: &AsyncMutex<HashMap<String, ServerToolInfo>>,
    result: Value,
) {
    if let Some(array) = result.get("tools").and_then(Value::as_array) {
        let mut cache = tool_cache.lock().await;
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
            server = %server_name,
            tool_count = cache.len(),
            "Refreshed tool cache from HTTP server"
        );
    }
}
