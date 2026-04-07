//! # Configuration Module
//!
//! This module handles all configuration loading, parsing, and validation for the MCP client.
//!
//! ## Configuration
//!
//! The client uses a single Postcard binary configuration file:
//!
//! - **`app.pc`** - All settings (providers, model, prompts, agent, server)
//!
//! ## Key Types
//!
//! - [`AppConfig`] - Main configuration struct
//! - [`ModelProviderConfig`] - API provider configuration (Gemini, OpenAI, Ollama)
//! - [`PromptsConfig`] - Configurable prompts for agent behavior
//! - [`ToolConfig`] - Tool definition synced from MCP servers
//! - [`ServerConfig`] - MCP server connection settings
//! - [`postcard_config::AppConfig`] - Postcard-based unified config

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

/// WASM Agent Configuration (minimal, no provider info)
pub mod wasm_config;

/// Migration stubs (TOML → Postcard no longer supported)
pub mod migration;

pub use crate::constants::{CONFIG_PATH, ENV_PATH};

pub use app::{AppConfig, DocServerConfig, PromptsConfig};
pub use error::ConfigError;
pub use provider::{ModelInfo, ModelProviderConfig};
pub use server::{ServerConfig, TransportType};
pub use tool::ToolConfig;
