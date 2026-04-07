//! Application constants
//!
//! Single source of truth for paths and other constants.

/// Unified Postcard configuration file path (project root)
pub const CONFIG_PATH: &str = "app.pc";

/// Environment file path (project root)
pub const ENV_PATH: &str = ".env";

/// Example environment file path (project root)
pub const ENV_EXAMPLE_PATH: &str = ".env.example";

/// Default Gemini API path (fallback when not specified in config)
pub const DEFAULT_GEMINI_API_PATH: &str = "v1beta/models";
