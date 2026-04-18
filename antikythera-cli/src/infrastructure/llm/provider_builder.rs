//! Provider builder — CLI's primary entry-point for constructing a `DynamicModelProvider`
//!
//! This module acts as the bridge between the application configuration layer and
//! the core `DynamicModelProvider` router.  It iterates over [`ModelProviderConfig`]
//! entries, creates the correct HTTP client for each via [`ProviderFactory`], and
//! registers them with the provider.
//!
//! Using this function instead of `DynamicModelProvider::from_configs` (which is
//! gated behind the `http-providers` feature in core) keeps all LLM HTTP
//! implementation knowledge inside `antikythera-cli`.

use antikythera_core::config::ModelProviderConfig;
use antikythera_core::infrastructure::model::{DynamicModelProvider, ModelError};
use tracing::info;

use super::factory::ProviderFactory;

/// Build a [`DynamicModelProvider`] from a slice of provider configurations.
///
/// For each provider config:
/// 1. A concrete client is created via [`ProviderFactory::create`].
/// 2. The list of supported model names is extracted from the config.
/// 3. The client is registered with the provider under the config's ID.
///
/// Returns `Err(ModelError::configuration)` if no providers are configured.
pub fn build_provider_from_configs(
    configs: &[ModelProviderConfig],
) -> Result<DynamicModelProvider, ModelError> {
    if configs.is_empty() {
        return Err(ModelError::provider_not_found("Tidak ada provider LLM yang dikonfigurasi"));
    }

    let mut provider = DynamicModelProvider::new();

    for config in configs {
        let models: Vec<String> = config.models.iter().map(|m| m.name.clone()).collect();

        if models.is_empty() {
            info!(
                provider = config.id.as_str(),
                "Provider dilewati: tidak ada model yang dikonfigurasi"
            );
            continue;
        }

        let client = ProviderFactory::create(config);

        info!(
            provider = config.id.as_str(),
            models = ?models,
            "Mendaftarkan provider LLM"
        );

        provider = provider.register(config.id.clone(), models, client);
    }

    Ok(provider)
}
