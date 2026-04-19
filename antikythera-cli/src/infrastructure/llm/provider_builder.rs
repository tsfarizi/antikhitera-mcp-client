//! Provider builder — CLI's primary entry-point for constructing a
//! `DynamicModelProvider` from configured providers.

use antikythera_core::config::ModelProviderConfig;
use antikythera_core::infrastructure::model::{DynamicModelProvider, ModelError};

use super::factory::ProviderFactory;

/// Build a [`DynamicModelProvider`] from a slice of provider configurations.
pub fn build_provider_from_configs(
    configs: &[ModelProviderConfig],
) -> Result<DynamicModelProvider, ModelError> {
    if configs.is_empty() {
        return Err(ModelError::provider_not_found("<empty-provider-config>"));
    }

    let provider = configs.iter().fold(DynamicModelProvider::new(), |provider, config| {
        let client = ProviderFactory::create(config);
        let models = config.models.iter().map(|m| m.name.clone()).collect();
        provider.register(config.id.clone(), models, client)
    });

    Ok(provider)
}
