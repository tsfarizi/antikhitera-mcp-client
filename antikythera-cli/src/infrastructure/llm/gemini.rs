//! Gemini LLM Provider
//!
//! Calls Google Gemini API directly.

use crate::domain::entities::*;
use crate::domain::use_cases::chat_use_case::LlmProvider;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: if model.is_empty() { "gemini-2.0-flash".to_string() } else { model },
            endpoint: "https://generativelanguage.googleapis.com".to_string(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn call(&self, messages: &[Message], system_prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.endpoint, self.model, self.api_key
        );

        // Build Gemini request format
        let contents: Vec<_> = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::Tool => "user",
                    MessageRole::System => "user",
                };
                json!({
                    "role": role,
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let request_body = json!({
            "contents": contents,
            "systemInstruction": {"parts": [{"text": system_prompt}]},
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 8192,
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
            return Err(format!("Gemini API error {}: {}", status, body).into());
        }

        let json: serde_json::Value = response.json().await?;

        // Parse Gemini response format
        let content = json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or("Invalid Gemini response structure")?
            .to_string();

        Ok(content)
    }
}
