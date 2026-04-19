//! Config loading for CLI
//!
//! Loads the shared `app.pc` config and converts it into CLI domain objects.

use crate::config::AppConfig;
use crate::domain::entities::*;
use std::error::Error;
use std::path::Path;

pub const CLI_CONFIG_PATH: &str = antikythera_core::config::postcard_config::CONFIG_PATH;

/// Load the shared config from `app.pc`.
pub fn load_cli_config(path: Option<&Path>) -> Result<AppConfig, Box<dyn Error + Send + Sync>> {
    crate::config::load_config(path).map_err(|e| e.into())
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

    Err(format!(
        "Direct model invocation for provider '{}' ({}) is disabled in this repository. The embedding host must call the model API and pass the response back into the framework.",
        provider.id,
        provider.provider_type
    )
    .into())
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

