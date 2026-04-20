//! Config loading for CLI
//!
//! Loads the shared `app.pc` config and converts it into CLI domain objects.

use crate::config::AppConfig;
use crate::domain::entities::*;
use crate::error::{CliError, CliResult};
use std::path::Path;

pub const CLI_CONFIG_PATH: &str = antikythera_core::config::postcard_config::CONFIG_PATH;

/// Load the shared config from `app.pc`.
pub fn load_app_config(path: Option<&Path>) -> CliResult<AppConfig> {
    crate::config::load_app_config(path)
}

/// Deprecated compatibility alias.
#[deprecated(
    since = "0.9.9",
    note = "use load_app_config instead; scheduled removal in 2.0.0"
)]
pub fn load_cli_config(path: Option<&Path>) -> CliResult<AppConfig> {
    load_app_config(path)
}

/// Create an [`LlmProvider`] box from the active provider in `config`.
pub fn build_llm_provider(
    config: &AppConfig,
) -> CliResult<Box<dyn crate::domain::use_cases::chat_use_case::LlmProvider>> {
    let provider = config
        .providers
        .iter()
        .find(|p| p.id == config.model.default_provider)
        .ok_or_else(|| {
            CliError::Validation(format!(
                "Provider '{}' not found",
                config.model.default_provider
            ))
        })?;

    Err(CliError::Unsupported(format!(
        "Direct model invocation for provider '{}' ({}) is disabled in this repository. The embedding host must call the model API and pass the response back into the framework.",
        provider.id, provider.provider_type
    )))
}

/// Deprecated compatibility alias.
#[deprecated(
    since = "0.9.9",
    note = "use build_llm_provider instead; scheduled removal in 2.0.0"
)]
pub fn create_llm_provider(
    config: &AppConfig,
) -> CliResult<Box<dyn crate::domain::use_cases::chat_use_case::LlmProvider>> {
    build_llm_provider(config)
}

/// Build a CLI [`ProviderConfig`] domain entity from the active provider in `config`.
pub fn build_active_provider_config(config: &AppConfig) -> CliResult<ProviderConfig> {
    let provider = config
        .providers
        .iter()
        .find(|p| p.id == config.model.default_provider)
        .ok_or_else(|| {
            CliError::Validation(format!(
                "Provider '{}' not found",
                config.model.default_provider
            ))
        })?;

    let provider_type = provider
        .provider_type
        .parse::<ProviderType>()
        .map_err(|_| {
            CliError::Validation(format!("Unknown provider type: {}", provider.provider_type))
        })?;

    Ok(ProviderConfig {
        id: provider.id.clone(),
        provider_type,
        endpoint: provider.endpoint.clone(),
        api_key: if provider.api_key.is_empty() {
            None
        } else {
            Some(provider.api_key.clone())
        },
        model: config.model.model.clone(),
    })
}

/// Deprecated compatibility alias.
#[deprecated(
    since = "0.9.9",
    note = "use build_active_provider_config instead; scheduled removal in 2.0.0"
)]
pub fn create_provider_config(config: &AppConfig) -> CliResult<ProviderConfig> {
    build_active_provider_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> AppConfig {
        AppConfig {
            providers: vec![crate::config::ProviderConfig {
                id: "ollama-local".to_string(),
                provider_type: "ollama".to_string(),
                endpoint: "http://127.0.0.1:11434".to_string(),
                api_key: String::new(),
                models: Vec::new(),
            }],
            model: crate::config::ModelConfig {
                default_provider: "ollama-local".to_string(),
                model: "llama3".to_string(),
            },
            ..AppConfig::default()
        }
    }

    #[test]
    fn build_active_provider_config_maps_provider() {
        let config = sample_config();
        let provider = build_active_provider_config(&config).expect("provider should map");
        assert_eq!(provider.id, "ollama-local");
        assert_eq!(provider.model, "llama3");
        assert!(provider.api_key.is_none());
    }

    #[test]
    fn build_llm_provider_returns_typed_unsupported_error() {
        let config = sample_config();
        match build_llm_provider(&config) {
            Ok(_) => panic!("expected unsupported error"),
            Err(err) => assert!(err.to_string().contains("unsupported operation")),
        }
    }

    #[test]
    fn alias_functions_delegate_to_new_names() {
        let config = sample_config();
        let p1 = build_active_provider_config(&config).expect("p1");
        #[allow(deprecated)]
        let p2 = create_provider_config(&config).expect("p2");
        assert_eq!(p1.id, p2.id);
    }
}
