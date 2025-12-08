//! Application constants
//!
//! Single source of truth for paths and other constants.

/// Default configuration file path
pub const CONFIG_PATH: &str = "config/client.toml";

/// Default environment file path
pub const ENV_PATH: &str = "config/.env";

/// Configuration directory
pub const CONFIG_DIR: &str = "config";

/// Default Gemini API path (fallback when not specified in config)
pub const DEFAULT_GEMINI_API_PATH: &str = "v1beta/models";
