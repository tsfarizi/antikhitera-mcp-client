use super::app::{PromptsConfig, RestServerConfig};
use super::error::ConfigError;
use super::provider::{ModelProviderConfig, RawProviderConfig};
use super::server::{RawServer, ServerConfig};
use super::tool::{RawTool, ToolConfig};
use super::{CONFIG_PATH, ENV_PATH};
use crate::constants::MODEL_CONFIG_PATH;
use dotenvy::from_filename;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Once;
use tracing::debug;

static ENV_LOADER: Once = Once::new();

/// Raw configuration structure for client.toml (providers, servers, REST settings)
#[derive(Debug, Deserialize, Default)]
struct RawClientConfig {
    #[serde(default)]
    pub servers: Vec<RawServer>,
    #[serde(default)]
    pub providers: Vec<RawProviderConfig>,
    /// REST server configuration
    #[serde(default)]
    pub server: RestServerConfig,
}

/// Raw configuration structure for model.toml (default_provider, model, tools, prompts)
#[derive(Debug, Deserialize, Default)]
struct RawModelConfig {
    pub model: Option<String>,
    pub default_provider: Option<String>,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<RawTool>,
    /// Optional prompts configuration section (includes template)
    #[serde(default)]
    pub prompts: PromptsConfig,
}

/// Ensures environment variables are loaded from config/.env
pub fn ensure_env_loaded() {
    ENV_LOADER.call_once(|| {
        let _ = from_filename(ENV_PATH);
    });
}

/// Load and validate configuration from file paths
/// Loads from both client.toml and model.toml
pub fn load_config(path: Option<&Path>) -> Result<super::AppConfig, ConfigError> {
    ensure_env_loaded();
    let client_path = path.unwrap_or_else(|| Path::new(CONFIG_PATH));

    // Derive model.toml path from client.toml's parent directory
    let model_path = if let Some(parent) = client_path.parent() {
        parent.join("model.toml")
    } else {
        Path::new(MODEL_CONFIG_PATH).to_path_buf()
    };

    read_configs(client_path, &model_path)
}

fn read_configs(client_path: &Path, model_path: &Path) -> Result<super::AppConfig, ConfigError> {
    // Read client.toml
    debug!(path = %client_path.display(), "Reading client configuration file");
    let client_content = fs::read_to_string(client_path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: client_path.to_path_buf(),
            }
        } else {
            ConfigError::Io {
                path: client_path.to_path_buf(),
                source,
            }
        }
    })?;
    let client_parsed: RawClientConfig =
        toml::from_str(&client_content).map_err(|source| ConfigError::Parse {
            path: client_path.to_path_buf(),
            source,
        })?;

    // Read model.toml
    debug!(path = %model_path.display(), "Reading model configuration file");
    let model_content = fs::read_to_string(model_path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: model_path.to_path_buf(),
            }
        } else {
            ConfigError::Io {
                path: model_path.to_path_buf(),
                source,
            }
        }
    })?;
    let model_parsed: RawModelConfig =
        toml::from_str(&model_content).map_err(|source| ConfigError::Parse {
            path: model_path.to_path_buf(),
            source,
        })?;

    validate_and_build(client_parsed, model_parsed)
}

fn validate_and_build(
    client: RawClientConfig,
    model: RawModelConfig,
) -> Result<super::AppConfig, ConfigError> {
    let model_name = model.model.ok_or(ConfigError::MissingModel)?;
    let default_provider = model
        .default_provider
        .ok_or(ConfigError::MissingDefaultProvider)?;

    if client.providers.is_empty() {
        return Err(ConfigError::NoProvidersConfigured);
    }

    let mut providers: Vec<ModelProviderConfig> = Vec::new();
    for raw_provider in client.providers {
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
        provider.ensure_model(&model_name);
    }

    Ok(super::AppConfig {
        default_provider,
        model: model_name,
        system_prompt: model.system_prompt,
        tools: model.tools.into_iter().map(ToolConfig::from).collect(),
        servers: client.servers.into_iter().map(ServerConfig::from).collect(),
        providers,
        rest_server: client.server,
        prompts: model.prompts,
    })
}
