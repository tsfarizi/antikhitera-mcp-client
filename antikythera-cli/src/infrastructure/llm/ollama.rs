//! Ollama LLM Provider
//!
//! Thin adapter over `antikythera_core`'s `OllamaClient`.  All HTTP logic and
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
        MessageRole::Tool => CoreRole::User,
    };
    ChatMessage::new(role, &m.content)
}

/// Ollama provider – delegates to core's `OllamaClient` via `ProviderFactory`.
pub struct OllamaProvider {
    client: Box<dyn ModelClient>,
    provider_id: String,
    model: String,
    /// Stored so `with_endpoint` can rebuild the client with the new base URL.
    #[allow(dead_code)]
    endpoint: String,
}

impl OllamaProvider {
    /// Create an Ollama provider pointing at the default local endpoint.
    pub fn new(model: String) -> Self {
        let endpoint = "http://127.0.0.1:11434".to_string();
        Self::with_endpoint_inner(model, endpoint)
    }

    fn with_endpoint_inner(model: String, endpoint: String) -> Self {
        let provider_id = "ollama".to_string();
        let model_name = if model.is_empty() {
            "llama3".to_string()
        } else {
            model
        };
        let config = ModelProviderConfig {
            id: provider_id.clone(),
            provider_type: "ollama".to_string(),
            endpoint: endpoint.clone(),
            api_key: None,
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
            endpoint,
        }
    }

    /// Override the Ollama server URL (e.g. for a remote instance).
    pub fn with_endpoint(self, endpoint: String) -> Self {
        Self::with_endpoint_inner(self.model, endpoint)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
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

