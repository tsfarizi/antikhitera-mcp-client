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
use crate::error::{CliError, CliResult};
pub use antikythera_core::config::postcard_config::{
    AgentConfig, AppConfig, CONFIG_PATH, DocServerConfig, ModelConfig, ModelInfo, PromptsConfig,
    ProviderConfig, ServerConfig,
};

/// Type alias kept for backward compatibility within the CLI crate.
/// Prefer using `AppConfig` directly in new code.
pub type CliConfig = AppConfig;

/// Type alias kept for backward compatibility within the CLI crate.
/// Prefer using `ProviderConfig` directly in new code.
pub type CliProviderConfig = ProviderConfig;

// ── Thin serialization wrappers ────────────────────────────────────────────────

use std::path::Path;

fn default_provider_catalog() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "ollama".to_string(),
            provider_type: "ollama".to_string(),
            endpoint: "http://127.0.0.1:11434".to_string(),
            api_key: String::new(),
            models: vec![ModelInfo {
                name: "llama3.2".to_string(),
                display_name: "Llama 3.2".to_string(),
            }],
        },
        ProviderConfig {
            id: "gemini".to_string(),
            provider_type: "gemini".to_string(),
            endpoint: "https://generativelanguage.googleapis.com".to_string(),
            api_key: "GEMINI_API_KEY".to_string(),
            models: vec![ModelInfo {
                name: "gemini-2.0-flash".to_string(),
                display_name: "Gemini 2.0 Flash".to_string(),
            }],
        },
        ProviderConfig {
            id: "openai".to_string(),
            provider_type: "openai".to_string(),
            endpoint: "https://api.openai.com".to_string(),
            api_key: "OPENAI_API_KEY".to_string(),
            models: vec![ModelInfo {
                name: "gpt-4o-mini".to_string(),
                display_name: "GPT-4o Mini".to_string(),
            }],
        },
    ]
}

pub fn recommended_default_config() -> AppConfig {
    AppConfig {
        providers: default_provider_catalog(),
        model: ModelConfig {
            default_provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
        },
        ..AppConfig::default()
    }
}

pub fn normalize_provider_type(provider_type: &str) -> String {
    match provider_type.trim().to_ascii_lowercase().as_str() {
        "google" | "google-ai" => "gemini".to_string(),
        "localai" => "ollama".to_string(),
        other => other.to_string(),
    }
}

pub fn default_models_for_provider(provider_type: &str) -> Vec<ModelInfo> {
    match normalize_provider_type(provider_type).as_str() {
        "gemini" => vec![ModelInfo {
            name: "gemini-2.0-flash".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
        }],
        "openai" => vec![ModelInfo {
            name: "gpt-4o-mini".to_string(),
            display_name: "GPT-4o Mini".to_string(),
        }],
        _ => vec![ModelInfo {
            name: "llama3.2".to_string(),
            display_name: "Llama 3.2".to_string(),
        }],
    }
}

/// Serialize `AppConfig` to Postcard binary.
pub fn config_to_postcard(config: &AppConfig) -> CliResult<Vec<u8>> {
    antikythera_core::config::postcard_config::config_to_postcard(config).map_err(CliError::Config)
}

/// Deserialize `AppConfig` from Postcard binary.
pub fn config_from_postcard(data: &[u8]) -> CliResult<AppConfig> {
    antikythera_core::config::postcard_config::config_from_postcard(data).map_err(CliError::Config)
}

/// Load `AppConfig` from `path` (defaults to [`CONFIG_PATH`] = `app.pc`).
pub fn load_app_config(path: Option<&Path>) -> CliResult<AppConfig> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));
    if !config_path.exists() {
        return Err(CliError::Config(format!(
            "Config not found: {}",
            config_path.display()
        )));
    }
    let data = std::fs::read(config_path)?;
    config_from_postcard(&data)
}

/// Deprecated compatibility alias.
#[deprecated(
    since = "0.9.9",
    note = "use load_app_config instead; scheduled removal in 2.0.0"
)]
pub fn load_config(path: Option<&Path>) -> CliResult<AppConfig> {
    load_app_config(path)
}

/// Save `AppConfig` to `path` (defaults to [`CONFIG_PATH`] = `app.pc`).
pub fn save_app_config(config: &AppConfig, path: Option<&Path>) -> CliResult<()> {
    let config_path = path.unwrap_or(Path::new(CONFIG_PATH));
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = config_to_postcard(config)?;
    std::fs::write(config_path, &data)?;
    Ok(())
}

/// Deprecated compatibility alias.
#[deprecated(
    since = "0.9.9",
    note = "use save_app_config instead; scheduled removal in 2.0.0"
)]
pub fn save_config(config: &AppConfig, path: Option<&Path>) -> CliResult<()> {
    save_app_config(config, path)
}

/// Returns `true` if the config file already exists at the default path.
pub fn config_exists() -> bool {
    Path::new(CONFIG_PATH).exists()
}

/// Create and persist a default `AppConfig` at [`CONFIG_PATH`].
pub fn init_default_config() -> CliResult<AppConfig> {
    let config = recommended_default_config();
    save_app_config(&config, None)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_postcard_uses_typed_result() {
        let config = AppConfig::default();
        let bytes = config_to_postcard(&config).expect("serialize");
        let decoded = config_from_postcard(&bytes).expect("deserialize");
        assert_eq!(
            decoded.model.default_provider,
            config.model.default_provider
        );
    }

    #[test]
    fn missing_file_returns_typed_error() {
        let missing = Path::new("definitely-not-exists-app.pc");
        let err = load_app_config(Some(missing)).expect_err("missing file should error");
        assert!(err.to_string().contains("configuration error"));
    }

    #[test]
    fn deprecated_aliases_delegate_to_new_names() {
        let missing = Path::new("definitely-not-exists-app.pc");
        let e1 = load_app_config(Some(missing))
            .expect_err("expected error")
            .to_string();
        #[allow(deprecated)]
        let e2 = load_config(Some(missing))
            .expect_err("expected error")
            .to_string();
        assert_eq!(e1, e2);
    }

    #[test]
    fn recommended_default_config_includes_primary_providers() {
        let config = recommended_default_config();
        let ids: Vec<&str> = config
            .providers
            .iter()
            .map(|provider| provider.id.as_str())
            .collect();
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"ollama"));
        assert_eq!(config.model.default_provider, "ollama");
    }

    #[test]
    fn normalize_provider_type_maps_known_aliases() {
        assert_eq!(normalize_provider_type("GEMINI"), "gemini");
        assert_eq!(normalize_provider_type("google-ai"), "gemini");
        assert_eq!(normalize_provider_type("LOCALAI"), "ollama");
        assert_eq!(normalize_provider_type("openai"), "openai");
    }
}
