//! Ollama client — CLI-side implementation
//!
//! Implements `ModelClient` for a locally-running Ollama instance.  This is
//! the CLI-owned version; the core crate is free of this HTTP dependency.

use antikythera_core::ProviderLogger;
use antikythera_core::infrastructure::model::traits::ModelClient;
use antikythera_core::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::super::adapter::MessageAdapter;
use super::super::http_client::HttpClientBase;
use super::super::streaming::{StreamAction, extract_stream_content};
use super::super::types::ModelProviderConfig;

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

        let force_json = request
            .params
            .get("output_format")
            .and_then(|v| v.as_str())
            .map(|s| s == "json")
            .unwrap_or(false);
        let payload = OllamaRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_ollama_format(&request.messages),
            stream: true,
            format: if force_json {
                Some("json".to_string())
            } else {
                None
            },
        };

        let log = ProviderLogger::new(
            request
                .session_id
                .as_deref()
                .unwrap_or(&antikythera_core::get_active_session()),
        );
        if force_json {
            log.debug("ModelParams detected output_format=json — Ollama format set to json");
        }
        log.info(format!(
            "Sending request to Ollama | provider={} model={} messages={}",
            self.base.id.as_str(),
            request.model.as_str(),
            request.messages.len()
        ));

        let raw = self.base.post_no_auth_text(&url, &payload).await?;
        log.debug("Received response from Ollama");

        let content = extract_stream_content(
            &raw,
            self.base.id.as_str(),
            request.session_id.as_deref(),
            |line| {
                let chunk = serde_json::from_str::<OllamaStreamChunk>(line).ok()?;
                if chunk.done.unwrap_or(false) {
                    return Some(StreamAction::Done);
                }
                chunk.message.map(|m| StreamAction::Chunk(m.content))
            },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
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
