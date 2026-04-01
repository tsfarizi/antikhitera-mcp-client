//! Ollama client implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};

/// Ollama client for local LLM
#[derive(Clone)]
pub struct OllamaClient {
    base: HttpClientBase,
}

impl OllamaClient {
    /// Creates client from provider config.
    pub fn from_config(config: &ModelProviderConfig) -> Self {
        Self {
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
            stream: false,
        };

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to Ollama"
        );

        let response: OllamaResponse = self.base.post_no_auth(&url, &payload).await?;
        debug!("Received response from Ollama");

        let content = response
            .message
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing message"))?
            .content;

        Ok(ModelResponse::new(content, request.session_id))
    }
}

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
