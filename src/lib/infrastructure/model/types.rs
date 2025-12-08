//! Model types - Request, Response, and Error types

use crate::types::{ChatMessage, MessageRole};
use reqwest::StatusCode;
use thiserror::Error;

/// Model request for LLM chat
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
}

/// Model response from LLM
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub message: ChatMessage,
    pub session_id: Option<String>,
}

impl ModelResponse {
    pub fn new(content: String, session_id: Option<String>) -> Self {
        Self {
            message: ChatMessage::new(MessageRole::Assistant, content),
            session_id,
        }
    }
}

/// Model errors
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("provider '{provider}' is not configured")]
    ProviderNotFound { provider: String },
    #[error("model '{model}' is not available for provider '{provider}'")]
    ModelNotFound { provider: String, model: String },
    #[error("provider '{provider}' requires an API key")]
    MissingApiKey { provider: String },
    #[error("network error calling provider '{provider}': {source}")]
    Network {
        provider: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("provider '{provider}' returned invalid response: {reason}")]
    InvalidResponse { provider: String, reason: String },
}

impl ModelError {
    pub fn provider_not_found(provider: impl Into<String>) -> Self {
        Self::ProviderNotFound {
            provider: provider.into(),
        }
    }

    pub fn model_not_found(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self::ModelNotFound {
            provider: provider.into(),
            model: model.into(),
        }
    }

    pub fn missing_api_key(provider: impl Into<String>) -> Self {
        Self::MissingApiKey {
            provider: provider.into(),
        }
    }

    pub fn network(provider: impl Into<String>, source: reqwest::Error) -> Self {
        Self::Network {
            provider: provider.into(),
            source,
        }
    }

    pub fn invalid_response(provider: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidResponse {
            provider: provider.into(),
            reason: reason.into(),
        }
    }

    /// User-friendly error message in Indonesian
    pub fn user_message(&self) -> String {
        match self {
            ModelError::ProviderNotFound { provider } => format!(
                "Penyedia model '{provider}' tidak ditemukan. Periksa pengaturan client.toml."
            ),
            ModelError::ModelNotFound { provider, model } => {
                format!("Model '{model}' tidak tersedia pada penyedia '{provider}'.")
            }
            ModelError::MissingApiKey { provider } => {
                format!("Penyedia '{provider}' memerlukan API key.")
            }
            ModelError::Network { provider, source } => {
                if source.is_connect() {
                    format!("Tidak dapat terhubung ke penyedia model '{provider}'.")
                } else if source.is_timeout() {
                    format!("Permintaan ke '{provider}' melebihi batas waktu.")
                } else if let Some(status) = source.status() {
                    match status {
                        StatusCode::NOT_FOUND => format!("Endpoint '{provider}' tidak ditemukan."),
                        StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => {
                            format!("Penyedia '{provider}' sedang tidak tersedia.")
                        }
                        _ => format!("Request ke '{provider}' gagal: {}", status.as_u16()),
                    }
                } else {
                    format!("Kesalahan jaringan pada '{provider}'.")
                }
            }
            ModelError::InvalidResponse { provider, .. } => {
                format!("Respons dari '{provider}' tidak valid.")
            }
        }
    }
}
