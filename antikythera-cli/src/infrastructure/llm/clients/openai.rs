//! OpenAI-compatible client — CLI-side implementation
//!
//! Implements `ModelClient` for any provider that exposes an OpenAI-compatible
//! chat completions API (OpenAI, Anthropic via proxy, Mistral, Groq, etc.).
//! This client is the CLI-owned version; the core crate is free of HTTP deps.

use antikythera_core::infrastructure::model::traits::ModelClient;

use super::super::types::ModelProviderConfig;
use antikythera_core::ProviderLogger;
use antikythera_core::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::super::adapter::MessageAdapter;
use super::super::factory::resolve_api_key;
use super::super::http_client::HttpClientBase;
use super::super::streaming::{StreamAction, extract_stream_content};

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

        let force_json = request
            .params
            .get("output_format")
            .and_then(|v| v.as_str())
            .map(|s| s == "json")
            .unwrap_or(false);
        let payload = OpenAIRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_openai_format(&request.messages),
            stream: true,
            response_format: if force_json {
                Some(ResponseFormat {
                    r#type: "json_object".to_string(),
                })
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
            log.debug("ModelParams detected output_format=json — OpenAI response_format set to json_object");
        }
        log.info(format!(
            "Sending request to OpenAI-compatible provider | provider={} model={} messages={}",
            self.base.id.as_str(),
            request.model.as_str(),
            request.messages.len()
        ));

        let raw = self.base.post_with_bearer_text(&url, &payload).await?;
        log.debug("Received response from OpenAI-compatible provider");

        let content = extract_stream_content(
            &raw,
            self.base.id.as_str(),
            request.session_id.as_deref(),
            |line| {
                if !line.starts_with("data:") {
                    return None;
                }
                let payload = line.trim_start_matches("data:").trim();
                if payload == "[DONE]" {
                    return Some(StreamAction::Done);
                }
                if payload.is_empty() {
                    return None;
                }
                serde_json::from_str::<OpenAIStreamChunk>(payload)
                    .ok()
                    .and_then(|chunk| {
                        chunk
                            .choices
                            .into_iter()
                            .find_map(|c| c.delta.and_then(|d| d.content))
                    })
                    .map(StreamAction::Chunk)
            },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
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
