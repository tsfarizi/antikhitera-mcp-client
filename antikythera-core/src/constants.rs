//! Application constants
//!
//! Single source of truth for paths and other constants.

/// Client configuration file path (providers, servers, REST settings)
pub const CONFIG_PATH: &str = "config/client.toml";

/// Model configuration file path (default_provider, model, prompt_template, tools)
pub const MODEL_CONFIG_PATH: &str = "config/model.toml";

/// Default environment file path
pub const ENV_PATH: &str = "config/.env";

/// Configuration directory
pub const CONFIG_DIR: &str = "config";

/// Default Gemini API path (fallback when not specified in config)
pub const DEFAULT_GEMINI_API_PATH: &str = "v1beta/models";
