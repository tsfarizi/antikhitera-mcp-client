use crate::types::{ChatMessage, MessageRole};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub message: ChatMessage,
    pub session_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("model provider returned invalid response: {0}")]
    InvalidResponse(String),
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}

#[derive(Clone)]
pub struct OllamaClient {
    http: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_client(base_url, Client::new())
    }

    pub fn with_client(base_url: impl Into<String>, client: Client) -> Self {
        Self {
            http: client,
            base_url: base_url.into(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{trimmed}/{path}")
    }
}

#[async_trait]
impl ModelProvider for OllamaClient {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.endpoint("/api/chat");
        let payload = OllamaChatRequest::from(&request);
        info!(
            model = request.model.as_str(),
            url = %url,
            messages = request.messages.len(),
            "Sending request to model provider"
        );
        let response: OllamaChatResponse = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        debug!("Received response from model provider");

        let message = response
            .message
            .ok_or_else(|| ModelError::InvalidResponse("missing message field".into()))?;

        let role = MessageRole::from_str(message.role.as_str())
            .ok_or_else(|| ModelError::InvalidResponse("unknown role in response".into()))?;

        Ok(ModelResponse {
            message: ChatMessage::new(role, message.content),
            session_id: request.session_id,
        })
    }
}

impl ModelError {
    pub fn user_message(&self) -> String {
        match self {
            ModelError::Network(err) => {
                if err.is_connect() {
                    "Tidak dapat terhubung ke layanan AI. Pastikan server Ollama berjalan dan dapat diakses."
                        .to_string()
                } else if err.is_timeout() {
                    "Permintaan ke layanan AI melebihi batas waktu. Coba lagi sebentar lagi."
                        .to_string()
                } else if let Some(status) = err.status() {
                    match status {
                        StatusCode::NOT_FOUND => {
                            "Endpoint AI tidak ditemukan (404). Periksa bahwa server Ollama menyediakan /api/chat."
                                .to_string()
                        }
                        StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => {
                            "Layanan AI sedang tidak tersedia. Coba lagi nanti.".to_string()
                        }
                        _ => format!(
                            "Permintaan ke layanan AI gagal dengan status {}. Coba lagi nanti.",
                            status.as_u16()
                        ),
                    }
                } else {
                    "Terjadi kesalahan jaringan saat menghubungi layanan AI. Coba lagi nanti."
                        .to_string()
                }
            }
            ModelError::InvalidResponse(_) => {
                "Layanan AI memberikan respons yang tidak dapat diproses. Coba lagi.".to_string()
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

impl From<&ModelRequest> for OllamaChatRequest {
    fn from(value: &ModelRequest) -> Self {
        Self {
            model: value.model.clone(),
            messages: value
                .messages
                .iter()
                .map(|msg| OllamaChatMessage {
                    role: msg.role.as_str().to_string(),
                    content: msg.content.clone(),
                })
                .collect(),
            stream: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_joins_paths_correctly() {
        let client = OllamaClient::new("http://localhost:11434/");
        assert_eq!(
            client.endpoint("/api/chat"),
            "http://localhost:11434/api/chat"
        );
    }

    #[test]
    fn request_conversion_preserves_roles() {
        let request = ModelRequest {
            model: "gemma3:4b".into(),
            messages: vec![
                ChatMessage::new(MessageRole::System, "stay concise"),
                ChatMessage::new(MessageRole::User, "hi"),
            ],
            session_id: None,
        };
        let payload = OllamaChatRequest::from(&request);
        let roles: Vec<_> = payload.messages.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["system", "user"]);
    }
}
