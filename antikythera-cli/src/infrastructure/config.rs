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
