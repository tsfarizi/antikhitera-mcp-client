//! Gemini client implementation

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, info};

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::constants::DEFAULT_GEMINI_API_PATH;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::factory::resolve_api_key;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse, ModelToolCall};

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

        if !request.tools.is_empty() {
            payload["tools"] = MessageAdapter::to_gemini_tools(&request.tools);
            if let Some(choice) = request.tool_choice.as_ref() {
                payload["toolConfig"] = MessageAdapter::to_gemini_tool_choice(choice);
            }
        }

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to Gemini"
        );

        let response: GeminiResponse = self.base.post_with_query_key(&url, &payload).await?;
        debug!("Received response from Gemini");

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for candidate in response.candidates.unwrap_or_default() {
            if let Some(content) = candidate.content {
                for part in content.parts {
                    if let Some(text) = part.text {
                        text_parts.push(text);
                    }
                    if let Some(function_call) = part.function_call {
                        tool_calls.push(ModelToolCall {
                            id: None,
                            name: function_call.name,
                            arguments: function_call.args.unwrap_or(serde_json::Value::Null),
                        });
                    }
                }
            }
        }

        if text_parts.is_empty() && tool_calls.is_empty() {
            return Err(ModelError::invalid_response(&self.base.id, "missing text or tool call"));
        }

        Ok(ModelResponse::with_details(
            text_parts.join(""),
            request.session_id,
            request.correlation_id,
            tool_calls,
            None,
        ))
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
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: Option<serde_json::Value>,
}
