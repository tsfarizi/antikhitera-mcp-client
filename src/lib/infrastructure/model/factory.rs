//! Provider factory - creates clients from config

use super::clients::{GeminiClient, OllamaClient, OpenAIClient};
use super::traits::ModelClient;
use crate::config::ModelProviderConfig;
use std::env;
use tracing::warn;

/// Infer API format from provider type string.
#[allow(dead_code)]
pub fn infer_api_format(provider_type: &str) -> String {
    match provider_type.to_lowercase().as_str() {
        "ollama" | "localai" => "ollama".to_string(),
        "gemini" | "google" | "google-ai" => "gemini".to_string(),
        _ => "openai".to_string(),
    }
}

/// Resolve API key from environment variable
pub fn resolve_api_key(provider: &str, spec: Option<&str>) -> Option<String> {
    let Some(raw) = spec.map(str::trim) else {
        return None;
    };
    if raw.is_empty() {
        return None;
    }
    match env::var(raw) {
        Ok(value) => Some(value),
        Err(err) => {
            warn!(
                provider,
                env_var = raw,
                %err,
                "API key environment variable is not set"
            );
            None
        }
    }
}

/// Factory for creating model clients from provider config.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Creates a model client based on provider type.
    ///
    /// Supported types:
    /// - `ollama`, `localai` → Ollama format
    /// - `gemini`, `google` → Gemini format  
    /// - Others → OpenAI-compatible format (default)
    pub fn create(config: &ModelProviderConfig) -> Box<dyn ModelClient> {
        match config.provider_type.to_lowercase().as_str() {
            "ollama" | "localai" => Box::new(OllamaClient::from_config(config)),
            "gemini" | "google" | "google-ai" => Box::new(GeminiClient::from_config(config)),
            _ => Box::new(OpenAIClient::from_config(config)),
        }
    }
}
