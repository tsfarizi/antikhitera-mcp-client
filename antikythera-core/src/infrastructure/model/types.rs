//! Model types - Request, Response, and Error types
//!
//! These types define the WASM-safe message contract between core and the host.
//! `ModelError::Network` intentionally uses a plain `String` so that `reqwest`
//! is not part of core's public API surface — HTTP error details are converted
//! to strings by the provider implementation layer (CLI or SDK) before
//! constructing this error.

use crate::domain::types::{ChatMessage, MessageRole};
use thiserror::Error;

/// Model request for LLM chat
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
    pub force_json: bool,
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
    /// Network / HTTP error.  The provider implementation converts the
    /// transport-layer error to a plain string so that `reqwest` is not
    /// referenced in core's public API surface.
    #[error("network error calling provider '{provider}': {message}")]
    Network { provider: String, message: String },
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

    /// Build a network error from any displayable error message.
    /// The caller (provider implementation) is responsible for converting
    /// transport-layer errors (e.g., `reqwest::Error`) to a string before
    /// calling this constructor.
    pub fn network(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Network {
            provider: provider.into(),
            message: message.into(),
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
            ModelError::Network { provider, message } => {
                // The provider implementation already stringified the transport error.
                format!("Kesalahan jaringan pada '{provider}': {message}")
            }
            ModelError::InvalidResponse { provider, .. } => {
                format!("Respons dari '{provider}' tidak valid.")
            }
        }
    }
}
