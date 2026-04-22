//! Dynamic model provider with multiple backends
//!
//! `DynamicModelProvider` is the routing layer that dispatches chat requests
//! to the correct `ModelClient` backend.  It is **always** available (no
//! feature gate) because it only contains pure routing logic with no HTTP
//! dependency.
//!
//! ## Building a provider
//!
//! Register host-backed `ModelClient` implementations directly:
//! ```no_run,ignore
//! let provider = DynamicModelProvider::new()
//!     .register("ollama", vec!["llama3".into()], Box::new(my_client));
//! ```
//! The host is responsible for providing the `ModelClient` implementations —
//! the core runtime only sees the trait.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

use super::traits::{ModelClient, ModelProvider};
use super::types::{ModelError, ModelRequest, ModelResponse};

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

/// Dynamic model provider that routes requests to appropriate backends.
///
/// This is a pure routing layer — it contains no HTTP client code.
/// Use [`register`](Self::register) to add backends directly.
#[derive(Default)]
pub struct DynamicModelProvider {
    backends: HashMap<String, ProviderRuntime>,
}

impl DynamicModelProvider {
    /// Create an empty provider with no registered backends.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a backend client for a given provider ID.
    ///
    /// `models` is the allow-list of model names accepted by this backend.
    /// Pass an empty `Vec` to accept any model name.
    ///
    /// # Example
    /// ```no_run,ignore
    /// let provider = DynamicModelProvider::new()
    ///     .register("ollama", vec![], Box::new(ollama_client))
    ///     .register("gemini", vec!["gemini-2.0-flash".into()], Box::new(gemini_client));
    /// ```
    pub fn register(
        mut self,
        id: impl Into<String>,
        models: Vec<String>,
        client: Box<dyn ModelClient>,
    ) -> Self {
        let runtime = ProviderRuntime {
            models: models.into_iter().collect(),
            client,
        };
        self.backends.insert(id.into(), runtime);
        self
    }

    /// Check if a backend for the given provider ID is registered.
    pub fn contains(&self, provider: &str) -> bool {
        self.backends.contains_key(provider)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
