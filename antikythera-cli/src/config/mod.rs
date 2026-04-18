//! CLI Configuration
//!
//! Delegates entirely to `antikythera_core::config::postcard_config` so that the
//! CLI binary and the core runtime share a **single** config file (`app.pc`) and
//! schema.  Previously the CLI maintained its own Postcard blob at `cli-config.pc`
//! with a divergent struct layout; that duplication has been removed.
//!
//! ## Migration note
//!
//! Existing `cli-config.pc` files are not automatically migrated.  Re-run
//! `antikythera-config init` (or the setup wizard via `antikythera --mode setup`)
//! to generate a fresh `app.pc`.

// Re-export the unified config types from core so the rest of the CLI crate and
// the `antikythera-config` binary can import them from a single place.
pub use antikythera_core::config::postcard_config::{
    AgentConfig,
    AppConfig,
    DocServerConfig,
    ModelConfig,
    ModelInfo,
    PromptsConfig,
    ProviderConfig,
    ServerConfig,
    CONFIG_PATH,
};

/// Type alias kept for backward compatibility within the CLI crate.
/// Prefer using `AppConfig` directly in new code.
pub type CliConfig = AppConfig;

/// Type alias kept for backward compatibility within the CLI crate.
/// Prefer using `ProviderConfig` directly in new code.
pub type CliProviderConfig = ProviderConfig;

// ── Thin serialization wrappers ────────────────────────────────────────────────

use std::path::Path;

/// Serialize `AppConfig` to Postcard binary.
pub fn config_to_postcard(config: &AppConfig) -> Result<Vec<u8>, String> {
    antikythera_core::config::postcard_config::config_to_postcard(config)
}

/// Deserialize `AppConfig` from Postcard binary.
pub fn config_from_postcard(data: &[u8]) -> Result<AppConfig, String> {
    antikythera_core::config::postcard_config::config_from_postcard(data)
}

/// Load `AppConfig` from `path` (defaults to [`CONFIG_PATH`] = `app.pc`).
pub fn load_config(path: Option<&Path>) -> Result<AppConfig, String> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));
    if !config_path.exists() {
        return Err(format!("Config not found: {}", config_path.display()));
    }
    let data =
        std::fs::read(config_path).map_err(|e| format!("Read error: {}", e))?;
    config_from_postcard(&data)
}

/// Save `AppConfig` to `path` (defaults to [`CONFIG_PATH`] = `app.pc`).
pub fn save_config(config: &AppConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create dir error: {}", e))?;
    }
    let data = config_to_postcard(config)?;
    std::fs::write(config_path, &data).map_err(|e| format!("Write error: {}", e))
}

/// Returns `true` if the config file already exists at the default path.
pub fn config_exists() -> bool {
    Path::new(CONFIG_PATH).exists()
}

/// Create and persist a default `AppConfig` at [`CONFIG_PATH`].
pub fn init_default_config() -> Result<AppConfig, String> {
    let config = AppConfig::default();
    save_config(&config, None)?;
    Ok(config)
}

