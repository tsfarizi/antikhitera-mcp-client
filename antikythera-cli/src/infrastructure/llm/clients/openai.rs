//! OpenAI-compatible client — CLI-side implementation
//!
//! Implements `ModelClient` for any provider that exposes an OpenAI-compatible
//! chat completions API (OpenAI, Anthropic via proxy, Mistral, Groq, etc.).
//! This client is the CLI-owned version; the core crate is free of HTTP deps.

use antikythera_core::config::ModelProviderConfig;
use antikythera_core::infrastructure::model::traits::ModelClient;
use antikythera_core::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::super::adapter::MessageAdapter;
use super::super::factory::resolve_api_key;
use super::super::http_client::HttpClientBase;
use super::super::streaming::{StreamEvent, emit_stream_event};

/// OpenAI-compatible client.
#[derive(Clone)]
pub struct OpenAIClient {
    base: HttpClientBase,
    api_path: String,
}

impl OpenAIClient {
    /// Construct from a provider configuration entry.
    pub fn from_config(config: &ModelProviderConfig) -> Self {
        let api_key = resolve_api_key(&config.id, config.api_key.as_deref());
        Self {
            base: HttpClientBase::new(config.id.clone(), config.endpoint.clone(), api_key),
            api_path: config
                .api_path
                .clone()
                .unwrap_or_else(|| "/v1/chat/completions".to_string()),
        }
    }
}

#[async_trait]
impl ModelClient for OpenAIClient {
    fn id(&self) -> &str {
        &self.base.id
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.base.build_url(&self.api_path);

        let payload = OpenAIRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_openai_format(&request.messages),
            stream: true,
        };

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to OpenAI-compatible provider"
        );

        let raw = self.base.post_with_bearer_text(&url, &payload).await?;
        debug!("Received response from OpenAI-compatible provider");

        let content = extract_openai_stream_content(
            &raw,
            self.base.id.as_str(),
            request.session_id.as_deref(),
        )
        .or_else(|| {
            serde_json::from_str::<OpenAIResponse>(&raw)
                .ok()
                .and_then(|response| {
                    response
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|c| c.message)
                        .map(|m| m.content)
                })
        })
        .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing content"))?;

        Ok(ModelResponse::new(content, request.session_id))
    }
}

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: Option<OpenAIMessage>,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: Option<OpenAIStreamDelta>,
}

#[derive(Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}

fn extract_openai_stream_content(
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
        if !trimmed.starts_with("data:") {
            continue;
        }
        let payload = trimmed.trim_start_matches("data:").trim();
        if payload == "[DONE]" {
            break;
        }
        if payload.is_empty() {
            continue;
        }

        if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(payload) {
            for choice in chunk.choices {
                if let Some(delta) = choice.delta
                    && let Some(piece) = delta.content
                {
                    saw_chunk = true;
                    emit_stream_event(StreamEvent::Chunk {
                        provider_id: provider_id.to_string(),
                        session_id: session.clone(),
                        content: piece.clone(),
                    });
                    content.push_str(&piece);
                }
            }
        }
    }

    emit_stream_event(StreamEvent::Completed {
        provider_id: provider_id.to_string(),
        session_id: session,
    });

    if saw_chunk { Some(content) } else { None }
}
