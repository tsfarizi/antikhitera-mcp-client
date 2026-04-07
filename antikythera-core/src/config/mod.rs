//! # Configuration Module
//!
//! This module handles all configuration loading, parsing, and validation for the MCP client.
//!
//! ## Configuration Files
//!
//! The client uses a split configuration approach:
//!
//! - **`client.toml`** - Contains provider settings, MCP servers, and REST server configuration
//! - **`model.toml`** - Contains model selection, prompt templates, and tool definitions
//!
//! ## Configuration Cache (Postcard)
//!
//! For faster loading, configurations are cached in Postcard binary format:
//!
//! - **First load**: TOML → Postcard cache
//! - **Subsequent loads**: Postcard cache directly (much faster)
//! - **On update**: TOML → Postcard cache (re-generated)
//!
//! ## Key Types
//!
//! - [`AppConfig`] - Main configuration struct loaded from both files
//! - [`ModelProviderConfig`] - API provider configuration (Gemini, OpenAI, Ollama)
//! - [`PromptsConfig`] - Configurable prompts for agent behavior
//! - [`ToolConfig`] - Tool definition synced from MCP servers
//! - [`ServerConfig`] - MCP server connection settings
//!
//! ## Example Configuration
//!
//! ```toml
//! # model.toml
//! default_provider = "gemini"
//! model = "gemini-2.0-flash"
//!
//! [prompts]
//! template = "You are a helpful assistant."
//! ```

pub mod cache;
pub mod app;
pub mod error;
pub mod loader;
pub mod provider;
pub mod serializer;
pub mod server;
pub mod tool;
pub mod wizard;

/// Unified Postcard-based configuration
pub mod postcard_config;

/// Migration from TOML to Postcard
pub mod migration;

pub use crate::constants::{CONFIG_PATH, ENV_PATH, MODEL_CONFIG_PATH};

pub use app::{AppConfig, DocServerConfig, PromptsConfig};
pub use error::ConfigError;
pub use provider::{ModelInfo, ModelProviderConfig};
pub use server::{ServerConfig, TransportType};
pub use tool::ToolConfig;
