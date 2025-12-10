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
//!
//! ## Loading Configuration
//!
//! ```no_run
//! use antikhitera_mcp_client::config::AppConfig;
//! use std::path::Path;
//!
//! let config = AppConfig::load(Some(Path::new("config/client.toml")))
//!     .expect("Failed to load config");
//! ```

pub mod app;
pub mod error;
pub mod loader;
pub mod provider;
pub mod serializer;
pub mod server;
pub mod tool;
pub mod wizard;

pub use crate::constants::{CONFIG_PATH, ENV_PATH, MODEL_CONFIG_PATH};

pub use app::{AppConfig, DocServerConfig, PromptsConfig};
pub use error::ConfigError;
pub use provider::{ModelInfo, ModelProviderConfig};
pub use server::ServerConfig;
pub use tool::ToolConfig;
