//! Provider factory — CLI-side
//!
//! Creates `ModelClient` instances from [`ModelProviderConfig`] entries.
//! This is the CLI-owned factory; keeping it here ensures that the
//! `antikythera-core` crate can be compiled as a WASM component without
//! pulling in any HTTP client machinery.
//!
//! [`ProviderFactory::create`] is the primary entry point, dispatching on
//! `provider_type` to instantiate the appropriate concrete client.

use std::path::Path;
use std::sync::Once;
use std::env;

use antikythera_core::infrastructure::model::traits::ModelClient;

use super::types::ModelProviderConfig;
use antikythera_core::ProviderLogger;

use super::clients::{GeminiClient, OllamaClient, OpenAIClient};

static CLI_ENV_LOADED: Once = Once::new();

/// Ensure `.env` from the CLI crate directory is loaded into the process
/// environment. Idempotent — subsequent calls are no-ops.
fn ensure_cli_env_loaded() {
    CLI_ENV_LOADED.call_once(|| {
        let manifest_env = Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
        if manifest_env.exists() {
            let _ = dotenvy::from_filename(&manifest_env);
        }
        // Always try CWD as a fallback.
        let _ = dotenvy::dotenv();
    });
}

/// Resolve an API key.
///
/// `spec` can be either:
/// - An environment-variable **name** (e.g. `"GEMINI_API_KEY"`) — resolved
///   via `std::env::var`.
/// - A **literal key value** (e.g. `"AIzaSy..."`) — returned directly.
///
/// An empty string or `None` returns `None`.
pub fn resolve_api_key(provider: &str, spec: Option<&str>) -> Option<String> {
    let raw = spec.map(str::trim)?;
    if raw.is_empty() {
        return None;
    }
    // Load .env from the CLI crate directory before attempting resolution.
    ensure_cli_env_loaded();

    // Try resolving as an environment-variable name.
    if let Ok(value) = env::var(raw) {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }
    // If `raw` doesn't look like an env-var name (all-uppercase with
    // underscores), it is likely a literal API key — return it directly.
    let is_env_var_name = raw.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        && raw.len() > 2
        && raw.contains('_');
    if !is_env_var_name {
        return Some(raw.to_string());
    }
    // It looks like an env-var name but wasn't set.
    ProviderLogger::new(&antikythera_core::get_active_session()).warn(format!(
        "API key environment variable is not set | provider={} env_var={}",
        provider, raw
    ));
    None
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
