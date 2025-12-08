//! Dynamic model provider with multiple backends

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

use super::factory::ProviderFactory;
use super::traits::{ModelClient, ModelProvider};
use super::types::{ModelError, ModelRequest, ModelResponse};
use crate::config::ModelProviderConfig;

/// Runtime container for a provider backend
struct ProviderRuntime {
    models: HashSet<String>,
    client: Box<dyn ModelClient>,
}

impl ProviderRuntime {
    fn supports(&self, model: &str) -> bool {
        self.models.is_empty() || self.models.contains(model)
    }
}

/// Dynamic model provider that routes requests to appropriate backends
#[derive(Default)]
pub struct DynamicModelProvider {
    backends: HashMap<String, ProviderRuntime>,
}

impl DynamicModelProvider {
    /// Create provider from config list using factory
    pub fn from_configs(configs: &[ModelProviderConfig]) -> Result<Self, ModelError> {
        let mut backends = HashMap::new();

        for config in configs {
            let models: HashSet<String> = config.models.iter().map(|m| m.name.clone()).collect();

            let client = ProviderFactory::create(config);

            backends.insert(config.id.clone(), ProviderRuntime { models, client });
        }

        Ok(Self { backends })
    }

    /// Check if provider exists
    pub fn contains(&self, provider: &str) -> bool {
        self.backends.contains_key(provider)
    }
}

#[async_trait]
impl ModelProvider for DynamicModelProvider {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let provider_id = &request.provider;

        let runtime = self
            .backends
            .get(provider_id)
            .ok_or_else(|| ModelError::provider_not_found(provider_id))?;

        if !runtime.supports(&request.model) {
            return Err(ModelError::model_not_found(provider_id, &request.model));
        }

        runtime.client.chat(request).await
    }
}
