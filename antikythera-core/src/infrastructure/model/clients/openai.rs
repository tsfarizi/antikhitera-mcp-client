//! OpenAI-compatible client implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::factory::resolve_api_key;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use crate::logging::ProviderLogger;

/// OpenAI-compatible client (works with OpenAI, Anthropic, Mistral, Groq, etc.)
#[derive(Clone)]
pub struct OpenAIClient {
    base: HttpClientBase,
    api_path: String,
}

impl OpenAIClient {
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ModelClient for OpenAIClient {
    fn id(&self) -> &str {
        &self.base.id
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.base.build_url(&self.api_path);

        let payload = OpenAIRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_openai_format(&request.messages),
            stream: false,
        };

        let log = ProviderLogger::new(
            request
                .session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );

        if let Some(last_msg) = request.messages.last() {
            let preview = crate::application::client::McpClient::<
                crate::infrastructure::model::DynamicModelProvider,
            >::summarise(&last_msg.content());
            log.info(format!(
                "-> OpenAI REQ | provider={} model={} messages={} | last_msg={}",
                self.base.id.as_str(),
                request.model.as_str(),
                request.messages.len(),
                preview
            ));
        }

        let response: OpenAIResponse = self.base.post_with_bearer(&url, &payload).await?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing content"))?;

        let preview = crate::application::client::McpClient::<
            crate::infrastructure::model::DynamicModelProvider,
        >::summarise(&content);
        log.info(format!(
            "<- OpenAI RES | chars={} | {}",
            content.len(),
            preview
        ));

        Ok(ModelResponse::new(content, request.session_id))
    }
}

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
