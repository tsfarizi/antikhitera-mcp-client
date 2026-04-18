//! Config loading for CLI
//!
//! Loads the shared `app.pc` config and converts it into CLI domain objects.

use crate::config::AppConfig;
use crate::domain::entities::*;
use antikythera_core::config::ModelProviderConfig;
use std::error::Error;
use std::path::Path;

pub const CLI_CONFIG_PATH: &str = antikythera_core::config::postcard_config::CONFIG_PATH;

/// Load the shared config from `app.pc`.
pub fn load_cli_config(path: Option<&Path>) -> Result<AppConfig, Box<dyn Error + Send + Sync>> {
    crate::config::load_config(path).map_err(|e| e.into())
}

/// Build a core [`ModelProviderConfig`] from a `postcard_config::ProviderConfig`.
///
/// The `api_key` field is treated as an environment-variable name per core's
/// convention.  Pass `None` or an empty string for providers that do not
/// require authentication (e.g. Ollama).
fn to_core_provider_config(p: &crate::config::ProviderConfig) -> ModelProviderConfig {
    ModelProviderConfig {
        id: p.id.clone(),
        provider_type: p.provider_type.clone(),
        endpoint: p.endpoint.clone(),
        api_key: if p.api_key.is_empty() { None } else { Some(p.api_key.clone()) },
        api_path: None,
        models: p.models.iter().map(|m| antikythera_core::config::ModelInfo {
            name: m.name.clone(),
            display_name: if m.display_name.is_empty() { None } else { Some(m.display_name.clone()) },
        }).collect(),
    }
}

/// Create an [`LlmProvider`] box from the active provider in `config`.
pub fn create_llm_provider(
    config: &AppConfig,
) -> Result<Box<dyn crate::domain::use_cases::chat_use_case::LlmProvider>, Box<dyn Error + Send + Sync>> {
    let provider = config
        .providers
        .iter()
        .find(|p| p.id == config.model.default_provider)
        .ok_or_else(|| format!("Provider '{}' not found", config.model.default_provider))?;

    let core_config = to_core_provider_config(provider);

    match provider.provider_type.to_lowercase().as_str() {
        "gemini" | "google" | "google-ai" => {
            let api_key = core_config.api_key.clone().unwrap_or_default();
            Ok(Box::new(
                crate::infrastructure::llm::GeminiProvider::new(api_key, config.model.model.clone()),
            ))
        }
        "ollama" | "localai" => {
            let endpoint = provider.endpoint.clone();
            Ok(Box::new(
                crate::infrastructure::llm::OllamaProvider::new(config.model.model.clone())
                    .with_endpoint(endpoint),
            ))
        }
        other => Err(format!("Unsupported provider type: {}", other).into()),
    }
}

/// Build a CLI [`ProviderConfig`] domain entity from the active provider in `config`.
pub fn create_provider_config(
    config: &AppConfig,
) -> Result<ProviderConfig, Box<dyn Error + Send + Sync>> {
    let provider = config
        .providers
        .iter()
        .find(|p| p.id == config.model.default_provider)
        .ok_or_else(|| format!("Provider '{}' not found", config.model.default_provider))?;

    let provider_type = ProviderType::from_str(&provider.provider_type)
        .ok_or_else(|| format!("Unknown provider type: {}", provider.provider_type))?;

    Ok(ProviderConfig {
        id: provider.id.clone(),
        provider_type,
        endpoint: provider.endpoint.clone(),
        api_key: if provider.api_key.is_empty() { None } else { Some(provider.api_key.clone()) },
        model: config.model.model.clone(),
    })
}

