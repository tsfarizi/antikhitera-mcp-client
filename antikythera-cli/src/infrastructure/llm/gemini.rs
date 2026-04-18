//! Gemini LLM Provider
//!
//! Thin adapter over `antikythera_core`'s `GeminiClient`.  All HTTP logic and
//! response parsing live in the core crate; this module only translates between
//! the CLI's `Message` type and the core's `ChatMessage` / `ModelRequest`.

use crate::domain::entities::{Message, MessageRole};
use crate::domain::use_cases::chat_use_case::LlmProvider;
use antikythera_core::config::{ModelInfo, ModelProviderConfig};
use antikythera_core::domain::types::{ChatMessage, MessageRole as CoreRole};
use antikythera_core::infrastructure::model::{traits::ModelClient, types::ModelRequest};
use crate::infrastructure::llm::factory::ProviderFactory;
use async_trait::async_trait;
use std::error::Error;

/// Converts a CLI `Message` to a core `ChatMessage`.
fn to_core_message(m: &Message) -> ChatMessage {
    let role = match m.role {
        MessageRole::User => CoreRole::User,
        MessageRole::Assistant => CoreRole::Assistant,
        MessageRole::System => CoreRole::System,
        // No Tool role in core's domain types – treat as user context.
        MessageRole::Tool => CoreRole::User,
    };
    ChatMessage::new(role, &m.content)
}

/// Gemini provider – delegates to core's `GeminiClient` via `ProviderFactory`.
pub struct GeminiProvider {
    client: Box<dyn ModelClient>,
    provider_id: String,
    model: String,
}

impl GeminiProvider {
    /// Create a Gemini provider.
    ///
    /// `api_key` should be an environment-variable name (e.g. `"GEMINI_API_KEY"`).
    /// The core factory resolves the variable at runtime.
    pub fn new(api_key: String, model: String) -> Self {
        let provider_id = "gemini".to_string();
        let model_name = if model.is_empty() {
            "gemini-2.0-flash".to_string()
        } else {
            model
        };
        let config = ModelProviderConfig {
            id: provider_id.clone(),
            provider_type: "gemini".to_string(),
            endpoint: "https://generativelanguage.googleapis.com".to_string(),
            api_key: if api_key.is_empty() { None } else { Some(api_key) },
            api_path: None,
            models: vec![ModelInfo {
                name: model_name.clone(),
                display_name: None,
            }],
        };
        Self {
            client: ProviderFactory::create(&config),
            provider_id,
            model: model_name,
        }
    }

    /// Override the endpoint (e.g. for testing or a Vertex AI proxy).
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        // Rebuild the client with the new endpoint.
        let config = ModelProviderConfig {
            id: self.provider_id.clone(),
            provider_type: "gemini".to_string(),
            endpoint,
            api_key: None,
            api_path: None,
            models: vec![ModelInfo {
                name: self.model.clone(),
                display_name: None,
            }],
        };
        self.client = ProviderFactory::create(&config);
        self
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn call(
        &self,
        messages: &[Message],
        system_prompt: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut core_messages: Vec<ChatMessage> =
            Vec::with_capacity(messages.len() + 1);

        if !system_prompt.is_empty() {
            core_messages.push(ChatMessage::new(CoreRole::System, system_prompt));
        }
        core_messages.extend(messages.iter().map(to_core_message));

        let request = ModelRequest {
            provider: self.provider_id.clone(),
            model: self.model.clone(),
            messages: core_messages,
            session_id: None,
            correlation_id: None,
            force_json: false,
            tools: Vec::new(),
            tool_choice: None,
        };

        let response = self.client.chat(request).await?;
        Ok(response.message.content())
    }
}

