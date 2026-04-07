use super::app::{PromptsConfig, RestServerConfig};
use super::cache::ConfigCacheManager;
use super::error::ConfigError;
use super::provider::{ModelProviderConfig, RawProviderConfig};
use super::server::{RawServer, ServerConfig};
use super::tool::{RawTool, ToolConfig};
use super::{CONFIG_PATH, ENV_PATH};
use dotenvy::from_filename;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;
use tracing::{debug, warn};

static ENV_LOADER: Once = Once::new();

/// Global cache manager instance (lazy-initialized)
static CACHE_MANAGER: OnceCell<ConfigCacheManager> = OnceCell::new();

/// Get or initialize the cache manager
fn get_cache_manager() -> &'static ConfigCacheManager {
    CACHE_MANAGER.get_or_init(|| {
        let cache_dir = PathBuf::from(".cache");
        ConfigCacheManager::new(cache_dir)
    })
}

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

/// Ensures environment variables are loaded from .env (project root)
pub fn ensure_env_loaded() {
    ENV_LOADER.call_once(|| {
        let _ = from_filename(ENV_PATH);
    });
}

/// Load and validate configuration from file paths
/// Uses Postcard cache for faster subsequent loads
pub fn load_config(path: Option<&Path>) -> Result<super::AppConfig, ConfigError> {
    ensure_env_loaded();
    let client_path = path.unwrap_or_else(|| Path::new(CONFIG_PATH));

    // Derive model.toml path from client.toml's parent directory
    let model_path = if let Some(parent) = client_path.parent() {
        parent.join("model.toml")
    } else {
        PathBuf::from("model.toml")
    };

    // Use cache for faster loading
    let cache_manager = get_cache_manager();
    
    // Try to load from cache first
    if cache_manager.cache_exists(client_path) {
        match cache_manager.load_from_cache::<super::AppConfig>(client_path) {
            Ok(config) => {
                return Ok(config);
            }
            Err(e) => {
                debug!(error = %e, "Cache load failed, will load from TOML");
                // Continue to load from TOML
            }
        }
    }

    // Load from TOML source
    debug!(
        client_path = %client_path.display(),
        model_path = %model_path.display(),
        "Loading configuration from TOML source"
    );
    
    let config = read_configs(client_path, &model_path)?;

    // Save to cache for next time
    if let Err(e) = cache_manager.save_to_cache(config.clone(), client_path) {
        warn!(error = %e, "Failed to save cache, will load from TOML next time");
    }

    Ok(config)
}

fn read_configs(
    client_path: &Path,
    model_path: &Path,
) -> Result<super::AppConfig, ConfigError> {
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
