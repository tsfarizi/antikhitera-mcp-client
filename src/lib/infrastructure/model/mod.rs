//! Model infrastructure module
//!
//! Provides LLM model providers with factory pattern for easy extensibility.
//!
//! # Structure
//! - `types` - Request, Response, Error types
//! - `traits` - ModelProvider, ModelClient traits
//! - `adapter` - Message format adapters
//! - `factory` - Provider factory for creating clients
//! - `clients` - Individual client implementations
//! - `provider` - DynamicModelProvider for routing

pub mod adapter;
pub mod clients;
pub mod factory;
pub mod provider;
pub mod traits;
pub mod types;

// Re-exports for convenience
pub use provider::DynamicModelProvider;
pub use traits::ModelProvider;
pub use types::{ModelError, ModelRequest, ModelResponse};
