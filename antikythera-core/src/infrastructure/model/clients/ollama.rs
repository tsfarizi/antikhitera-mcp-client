//! Ollama client implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::logging::ProviderLogger;

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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

        let log = ProviderLogger::new(
            request
                .session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );

        // Log the last user message content for IO trace
        if let Some(last_msg) = request.messages.last() {
            let preview = crate::application::client::McpClient::<crate::infrastructure::model::DynamicModelProvider>::summarise(&last_msg.content());
            log.info(format!(
                "→ Ollama REQ | provider={} model={} messages={} | last_msg={}",
                self.base.id.as_str(),
                request.model.as_str(),
                request.messages.len(),
                preview
            ));
        }

        let response: OllamaResponse = self.base.post_no_auth(&url, &payload).await?;

        let content = response
            .message
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing message"))?
            .content;

        let preview = crate::application::client::McpClient::<crate::infrastructure::model::DynamicModelProvider>::summarise(&content);
        log.info(format!(
            "← Ollama RES | chars={} | {}",
            content.len(),
            preview
        ));

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
