use super::CONFIG_PATH;
use super::error::ConfigError;
use super::provider::{ModelProviderConfig, RawProviderConfig};
use super::server::{RawServer, ServerConfig};
use super::tool::{RawTool, ToolConfig};
use dotenvy::from_filename;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Once;
use tracing::debug;

static ENV_LOADER: Once = Once::new();

/// Raw configuration structure for deserialization from TOML
#[derive(Debug, Deserialize, Default)]
pub(super) struct RawConfig {
    pub model: Option<String>,
    pub default_provider: Option<String>,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<RawTool>,
    #[serde(default)]
    pub servers: Vec<RawServer>,
    pub prompt_template: Option<String>,
    #[serde(default)]
    pub providers: Vec<RawProviderConfig>,
}

/// Ensures environment variables are loaded from config/.env
pub fn ensure_env_loaded() {
    ENV_LOADER.call_once(|| {
        let _ = from_filename("config/.env");
    });
}

/// Load and validate configuration from a file path
pub fn load_config(path: Option<&Path>) -> Result<super::AppConfig, ConfigError> {
    ensure_env_loaded();
    let config_path = path.unwrap_or_else(|| Path::new(CONFIG_PATH));
    read_config(config_path)
}

fn read_config(path: &Path) -> Result<super::AppConfig, ConfigError> {
    debug!(path = %path.display(), "Reading client configuration file");

    let content = fs::read_to_string(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: path.to_path_buf(),
            }
        } else {
            ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;

    let parsed: RawConfig = toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    validate_and_build(parsed)
}

fn validate_and_build(parsed: RawConfig) -> Result<super::AppConfig, ConfigError> {
    let model = parsed.model.ok_or(ConfigError::MissingModel)?;
    let default_provider = parsed
        .default_provider
        .ok_or(ConfigError::MissingDefaultProvider)?;
    let prompt_template = parsed
        .prompt_template
        .ok_or(ConfigError::MissingPromptTemplate)?;

    if parsed.providers.is_empty() {
        return Err(ConfigError::NoProvidersConfigured);
    }

    let mut providers: Vec<ModelProviderConfig> = Vec::new();
    for raw_provider in parsed.providers {
        if raw_provider.endpoint.is_none() {
            return Err(ConfigError::MissingEndpoint {
                provider: raw_provider.id.clone(),
            });
        }
        providers.push(ModelProviderConfig::from(raw_provider));
    }
    if !providers.iter().any(|p| p.id == default_provider) {
        return Err(ConfigError::ProviderNotFound {
            provider: default_provider,
        });
    }
    if let Some(provider) = providers.iter_mut().find(|p| p.id == default_provider) {
        provider.ensure_model(&model);
    }

    Ok(super::AppConfig {
        default_provider,
        model,
        system_prompt: parsed.system_prompt,
        tools: parsed.tools.into_iter().map(ToolConfig::from).collect(),
        servers: parsed.servers.into_iter().map(ServerConfig::from).collect(),
        prompt_template,
        providers,
    })
}
