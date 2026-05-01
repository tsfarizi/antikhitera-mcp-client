//! Provider factory — CLI-side
//!
//! Creates `ModelClient` instances from [`ModelProviderConfig`] entries.
//! This is the CLI-owned factory; keeping it here ensures that the
//! `antikythera-core` crate can be compiled as a WASM component without
//! pulling in any HTTP client machinery.
//!
//! [`ProviderFactory::create`] is the primary entry point, dispatching on
//! `provider_type` to instantiate the appropriate concrete client.

use antikythera_core::infrastructure::model::traits::ModelClient;

use super::types::ModelProviderConfig;
use antikythera_core::ProviderLogger;
use std::env;

use super::clients::{GeminiClient, OllamaClient, OpenAIClient};

/// Resolve an API key.
///
/// The `spec` value is expected to be the **name of an environment variable**
/// (e.g. `"GEMINI_API_KEY"`).  An empty string or `None` spec returns `None`.
pub fn resolve_api_key(provider: &str, spec: Option<&str>) -> Option<String> {
    let raw = spec.map(str::trim)?;
    if raw.is_empty() {
        return None;
    }
    match env::var(raw) {
        Ok(value) => Some(value),
        Err(err) => {
            ProviderLogger::new(&antikythera_core::get_active_session()).warn(format!(
                "API key environment variable is not set | provider={} env_var={} error={}",
                provider, raw, err
            ));
            None
        }
    }
}

/// Factory for creating `ModelClient` instances from provider configuration.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create the correct client for the given provider configuration.
    ///
    /// | `provider_type`          | Client               |
    /// |--------------------------|----------------------|
    /// | `"ollama"`, `"localai"`  | [`OllamaClient`]     |
    /// | `"gemini"`, `"google"`   | [`GeminiClient`]     |
    /// | anything else            | [`OpenAIClient`]     |
    pub fn create(config: &ModelProviderConfig) -> Box<dyn ModelClient> {
        match config.provider_type.to_lowercase().as_str() {
            "ollama" | "localai" => Box::new(OllamaClient::from_config(config)),
            "gemini" | "google" | "google-ai" => Box::new(GeminiClient::from_config(config)),
            _ => Box::new(OpenAIClient::from_config(config)),
        }
    }
}
