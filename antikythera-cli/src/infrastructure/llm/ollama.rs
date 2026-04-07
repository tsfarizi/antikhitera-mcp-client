//! Ollama LLM Provider
//!
//! Calls local Ollama API.

use crate::domain::entities::*;
use crate::domain::use_cases::chat_use_case::LlmProvider;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub struct OllamaProvider {
    client: Client,
    model: String,
    endpoint: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self {
            client: Client::new(),
            model: if model.is_empty() { "llama3".to_string() } else { model },
            endpoint: "http://127.0.0.1:11434".to_string(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn call(&self, messages: &[Message], system_prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/api/chat", self.endpoint);

        // Build Ollama request format
        let ollama_messages: Vec<_> = std::iter::once(Message::system(system_prompt))
            .chain(messages.iter().cloned())
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                    MessageRole::Tool => "user",
                };
                json!({
                    "role": role,
                    "content": m.content
                })
            })
            .collect();

        let request_body = json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 8192,
            }
        });

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Ollama API error {}: {}", status, body).into());
        }

        let json: serde_json::Value = response.json().await?;

        // Parse Ollama response format
        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or("Invalid Ollama response structure")?
            .to_string();

        Ok(content)
    }
}
