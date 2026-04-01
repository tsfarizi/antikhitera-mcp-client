//! JSON-RPC communication over HTTP.
//!
//! Handles sending JSON-RPC requests and notifications.

use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

use crate::application::tooling::error::ToolInvokeError;

/// MCP Protocol version
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Build headers for HTTP request, filtering empty Authorization headers.
pub fn build_headers(headers: &HashMap<String, String>) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(key, value)| {
            if key.eq_ignore_ascii_case("Authorization") {
                !value.trim().is_empty() && !value.trim().eq_ignore_ascii_case("Bearer")
            } else {
                true
            }
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Send a JSON-RPC request and return the result.
pub async fn send_request(
    client: &Client,
    server_name: &str,
    url: &str,
    method: &str,
    params: Value,
    headers: &HashMap<String, String>,
    id_counter: &AtomicU64,
) -> Result<Value, ToolInvokeError> {
    let id = id_counter.fetch_add(1, Ordering::SeqCst);
    let request_id = format!("req-{}", id);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    });

    info!(
        server = %server_name,
        method = method,
        url = %url,
        request_id = %request_id,
        "Sending HTTP JSON-RPC request"
    );

    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload);

    // Add custom headers
    for (key, value) in build_headers(headers) {
        request = request.header(&key, &value);
    }

    let response = request.send().await.map_err(|e| {
        warn!(
            server = %server_name,
            error = %e,
            "HTTP request failed"
        );
        ToolInvokeError::Transport {
            server: server_name.to_string(),
            message: format!("HTTP request failed: {}", e),
        }
    })?;

    let status = response.status();
    if !status.is_success() {
        warn!(
            server = %server_name,
            status = %status,
            "HTTP request returned error status"
        );
        return Err(ToolInvokeError::Transport {
            server: server_name.to_string(),
            message: format!("HTTP error: {}", status),
        });
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| ToolInvokeError::Transport {
            server: server_name.to_string(),
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
        warn!(
            server = %server_name,
            code = code,
            error_message = %message,
            "JSON-RPC error received"
        );
        return Err(ToolInvokeError::Rpc {
            server: server_name.to_string(),
            code,
            message,
        });
    }

    let result = body.get("result").cloned().unwrap_or(Value::Null);
    debug!(
        server = %server_name,
        "HTTP JSON-RPC request completed successfully"
    );
    Ok(result)
}

/// Send a JSON-RPC notification (no response expected).
pub async fn send_notification(
    client: &Client,
    server_name: &str,
    url: &str,
    method: &str,
    params: Value,
    headers: &HashMap<String, String>,
) -> Result<(), ToolInvokeError> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });

    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload);

    for (key, value) in build_headers(headers) {
        request = request.header(&key, &value);
    }

    let _ = request
        .send()
        .await
        .map_err(|e| ToolInvokeError::Transport {
            server: server_name.to_string(),
            message: format!("HTTP notification failed: {}", e),
        })?;

    Ok(())
}
