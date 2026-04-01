//! SSE (Server-Sent Events) handling for HTTP transport.
//!
//! Handles SSE connection establishment and session endpoint resolution.

use reqwest::Client;
use reqwest_eventsource::{Event, RequestBuilderExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

use crate::application::tooling::error::ToolInvokeError;

/// Timeout in seconds for SSE endpoint event
pub const SSE_TIMEOUT_SECS: u64 = 5;

/// Start SSE listener in background task.
///
/// Spawns a tokio task that listens for SSE events and updates
/// the session endpoint when received.
pub fn start_sse_listener(
    client: Client,
    name: String,
    url: String,
    headers: HashMap<String, String>,
    session_endpoint: Arc<AsyncMutex<Option<String>>>,
) {
    tokio::spawn(async move {
        debug!(server = %name, %url, "Starting SSE listener");

        let mut request = client.get(&url);

        // Add custom headers
        for (key, value) in &headers {
            if key.eq_ignore_ascii_case("Authorization") {
                if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("Bearer") {
                    continue;
                }
            }
            request = request.header(key, value);
        }

        let mut es = request.eventsource().unwrap();

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    info!(server = %name, "SSE connection opened");
                }
                Ok(Event::Message(message)) => {
                    debug!(server = %name, event = %message.event, "Received SSE event");
                    if message.event == "endpoint" {
                        let endpoint = message.data.trim().to_string();
                        info!(server = %name, %endpoint, "Received session endpoint");
                        *session_endpoint.lock().await = Some(endpoint);
                    }
                }
                Err(err) => {
                    warn!(server = %name, %err, "Error in SSE stream");
                }
            }
        }
        warn!(server = %name, "SSE stream ended");
    });
}

/// Resolve the session endpoint URL.
///
/// Waits for the SSE endpoint event with timeout, then resolves
/// relative URLs against the base URL.
pub async fn resolve_endpoint(
    name: &str,
    base_url: &str,
    session_endpoint: &AsyncMutex<Option<String>>,
) -> Result<String, ToolInvokeError> {
    let start = tokio::time::Instant::now();
    loop {
        if let Some(endpoint) = session_endpoint.lock().await.as_ref() {
            // Handle relative URLs
            if endpoint.starts_with("http") {
                return Ok(endpoint.clone());
            } else {
                // Join relative endpoint with base URL
                let url =
                    reqwest::Url::parse(base_url).map_err(|e| ToolInvokeError::Transport {
                        server: name.to_string(),
                        message: format!("Invalid base URL: {}", e),
                    })?;

                let joined = url.join(endpoint).map_err(|e| ToolInvokeError::Transport {
                    server: name.to_string(),
                    message: format!("Failed to join endpoint: {}", e),
                })?;
                return Ok(joined.to_string());
            }
        }

        if start.elapsed() > std::time::Duration::from_secs(SSE_TIMEOUT_SECS) {
            return Err(ToolInvokeError::Transport {
                server: name.to_string(),
                message: "Timed out waiting for session endpoint".to_string(),
            });
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
