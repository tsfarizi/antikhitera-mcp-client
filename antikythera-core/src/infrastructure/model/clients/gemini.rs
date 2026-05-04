//! Gemini client implementation

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::constants::DEFAULT_GEMINI_API_PATH;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::factory::resolve_api_key;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
use crate::logging::ProviderLogger;

/// Gemini client for Google AI
#[derive(Clone)]
pub struct GeminiClient {
    base: HttpClientBase,
    api_path: String,
}

impl GeminiClient {
    pub fn from_config(config: &ModelProviderConfig) -> Self {
        let api_key = resolve_api_key(&config.id, config.api_key.as_deref());
        Self {
            base: HttpClientBase::new(config.id.clone(), config.endpoint.clone(), api_key),
            api_path: config
                .api_path
                .clone()
                .unwrap_or_else(|| DEFAULT_GEMINI_API_PATH.to_string()),
        }
    }

    fn build_model_url(&self, model: &str) -> String {
        let base = self.base.endpoint.trim_end_matches('/');
        format!("{base}/{}/{model}:generateContent", self.api_path)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ModelClient for GeminiClient {
    fn id(&self) -> &str {
        &self.base.id
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.build_model_url(&request.model);
        let (system_text, contents) = MessageAdapter::to_gemini_format(&request.messages);

        let mut payload = json!({
            "contents": contents
        });

        if let Some(system) = system_text {
            payload["system_instruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        if request.force_json {
            payload["generationConfig"] = json!({
                "responseMimeType": "application/json"
            });
        }

        let log = ProviderLogger::new(
            request
                .session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );

        // Log the last user message content for IO trace
        if let Some(last_msg) = request.messages.last() {
            let preview = crate::application::client::McpClient::<
                crate::infrastructure::model::DynamicModelProvider,
            >::summarise(&last_msg.content());
            log.info(format!(
                "-> Gemini REQ | provider={} model={} messages={} | last_msg={}",
                self.base.id.as_str(),
                request.model.as_str(),
                request.messages.len(),
                preview
            ));
        }

        let response: GeminiResponse = self.base.post_with_query_key(&url, &payload).await?;

        let content = response
            .candidates
            .unwrap_or_default()
            .into_iter()
            .flat_map(|c| c.content)
            .flat_map(|c| c.parts)
            .find_map(|p| p.text)
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing text"))?;

        let preview = crate::application::client::McpClient::<
            crate::infrastructure::model::DynamicModelProvider,
        >::summarise(&content);
        log.info(format!(
            "<- Gemini RES | chars={} | {}",
            content.len(),
            preview
        ));

        Ok(ModelResponse::new(content, request.session_id))
    }
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
}
