//! Model traits

use super::types::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;

/// Trait for model provider implementations
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Send a chat request to the model provider
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}

/// Trait for individual model clients
#[async_trait]
pub trait ModelClient: Send + Sync {
    /// Get the client ID
    fn id(&self) -> &str;

    /// Send a chat request
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}
