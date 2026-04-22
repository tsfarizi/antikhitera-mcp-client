//! Ollama client — CLI-side implementation
//!
//! Implements `ModelClient` for a locally-running Ollama instance.  This is
//! the CLI-owned version; the core crate is free of this HTTP dependency.

use antikythera_core::infrastructure::model::traits::ModelClient;

use super::super::types::ModelProviderConfig;
use antikythera_core::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::super::adapter::MessageAdapter;
use super::super::http_client::HttpClientBase;
use super::super::streaming::{StreamEvent, emit_stream_event};

/// Ollama client for a local LLM inference server.
#[derive(Clone)]
pub struct OllamaClient {
    base: HttpClientBase,
}

impl OllamaClient {
    /// Construct from a provider configuration entry.
    pub fn from_config(config: &ModelProviderConfig) -> Self {
        Self {
            // Ollama does not use an API key.
            base: HttpClientBase::new(config.id.clone(), config.endpoint.clone(), None),
        }
    }
}

#[async_trait]
impl ModelClient for OllamaClient {
    fn id(&self) -> &str {
        &self.base.id
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.base.build_url("/api/chat");

        let payload = OllamaRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_ollama_format(&request.messages),
            stream: true,
        };

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to Ollama"
        );

        let raw = self.base.post_no_auth_text(&url, &payload).await?;
        debug!("Received response from Ollama");

        let content = extract_ollama_stream_content(
            &raw,
            self.base.id.as_str(),
            request.session_id.as_deref(),
        )
        .or_else(|| {
            serde_json::from_str::<OllamaResponse>(&raw)
                .ok()
                .and_then(|response| response.message.map(|m| m.content))
        })
        .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing message"))?;

        Ok(ModelResponse::new(content, request.session_id))
    }
}

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize)]
struct OllamaStreamChunk {
    message: Option<OllamaMessage>,
    done: Option<bool>,
}

fn extract_ollama_stream_content(
    raw: &str,
    provider_id: &str,
    session_id: Option<&str>,
) -> Option<String> {
    let mut content = String::new();
    let mut saw_chunk = false;
    let session = session_id.map(|v| v.to_string());

    emit_stream_event(StreamEvent::Started {
        provider_id: provider_id.to_string(),
        session_id: session.clone(),
    });

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(trimmed) {
            if let Some(msg) = chunk.message {
                saw_chunk = true;
                emit_stream_event(StreamEvent::Chunk {
                    provider_id: provider_id.to_string(),
                    session_id: session.clone(),
                    content: msg.content.clone(),
                });
                content.push_str(&msg.content);
            }
            if chunk.done.unwrap_or(false) {
                break;
            }
        }
    }

    emit_stream_event(StreamEvent::Completed {
        provider_id: provider_id.to_string(),
        session_id: session,
    });

    if saw_chunk { Some(content) } else { None }
}
