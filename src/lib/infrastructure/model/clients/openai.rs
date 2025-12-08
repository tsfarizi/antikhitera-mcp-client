//! OpenAI-compatible client implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::factory::resolve_api_key;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};

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
            stream: false,
        };

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to OpenAI-compatible provider"
        );

        let response: OpenAIResponse = self.base.post_with_bearer(&url, &payload).await?;
        debug!("Received response from OpenAI-compatible provider");

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing content"))?;

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
