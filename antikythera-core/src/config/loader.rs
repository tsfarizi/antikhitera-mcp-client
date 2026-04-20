//! Configuration loader - Postcard-only
//!
//! All configuration is stored as a single Postcard binary file (`app.pc`).

use super::app::{PromptsConfig, RestServerConfig};
use super::error::ConfigError;
use super::postcard_config;
use super::provider::ModelProviderConfig;
use crate::logging::ConfigLogger;
use dotenvy::from_filename;
use std::path::Path;
use std::sync::Once;

static ENV_LOADER: Once = Once::new();

/// Ensures environment variables are loaded from .env (project root)
pub fn ensure_env_loaded() {
    ENV_LOADER.call_once(|| {
        let _ = from_filename(super::ENV_PATH);
    });
}

/// Load and validate configuration from Postcard binary
pub fn load_config(path: Option<&Path>) -> Result<super::AppConfig, ConfigError> {
    ensure_env_loaded();

    let config_path = path.unwrap_or_else(|| Path::new(postcard_config::CONFIG_PATH));

    if !config_path.exists() {
        return Err(ConfigError::NotFound {
            path: config_path.to_path_buf(),
        });
    }

    let data = std::fs::read(config_path).map_err(|e| ConfigError::Io {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    let config = postcard_config::config_from_postcard(&data)
        .map_err(|e| ConfigError::CacheError(format!("Postcard deserialize error: {}", e)))?;

    // Log successful load
    let logger = ConfigLogger::new("config");
    logger.info(format!("Config loaded from: {}", config_path.display()));
    logger.debug(format!("  Providers: {}", config.providers.len()));
    logger.debug(format!(
        "  Default: {}/{}",
        config.model.default_provider, config.model.model
    ));

    Ok(convert_to_app_config(&config))
}

/// Save configuration to Postcard binary
pub fn save_config(config: &super::AppConfig, path: Option<&Path>) -> Result<(), ConfigError> {
    let config_path = path.unwrap_or_else(|| Path::new(postcard_config::CONFIG_PATH));

    let pc_config = convert_to_postcard_config(config);
    let data = postcard_config::config_to_postcard(&pc_config)
        .map_err(|e| ConfigError::CacheError(format!("Postcard serialize error: {}", e)))?;

    if let Some(parent) = config_path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
            path: config_path.to_path_buf(),
            source: e,
        })?;
    }

    std::fs::write(config_path, &data).map_err(|e| ConfigError::Io {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    // Log successful save
    let logger = ConfigLogger::new("config");
    logger.info(format!("Config saved to: {}", config_path.display()));
    logger.debug(format!("  Size: {} bytes", data.len()));

    Ok(())
}

/// Convert Postcard config to AppConfig
fn convert_to_app_config(pc: &postcard_config::PostcardAppConfig) -> super::AppConfig {
    super::AppConfig {
        default_provider: pc.model.default_provider.clone(),
        model: pc.model.model.clone(),
        system_prompt: None,
        tools: Vec::new(),
        servers: Vec::new(),
        providers: pc
            .providers
            .iter()
            .map(|p| ModelProviderConfig {
                id: p.id.clone(),
                provider_type: p.provider_type.clone(),
                endpoint: p.endpoint.clone(),
                api_key: if p.api_key.is_empty() {
                    None
                } else {
                    Some(p.api_key.clone())
                },
                api_path: None,
                models: p
                    .models
                    .iter()
                    .map(|m| crate::config::provider::ModelInfo {
                        name: m.name.clone(),
                        display_name: if m.display_name.is_empty() {
                            None
                        } else {
                            Some(m.display_name.clone())
                        },
                    })
                    .collect(),
            })
            .collect(),
        rest_server: RestServerConfig {
            bind: pc.server.bind.clone(),
            cors_origins: pc.server.cors_origins.clone(),
            docs: pc
                .server
                .docs
                .iter()
                .map(|d| crate::config::app::DocServerConfig {
                    url: d.url.clone(),
                    description: d.description.clone(),
                })
                .collect(),
        },
        prompts: PromptsConfig {
            template: opt_nonempty(&pc.prompts.template),
            tool_guidance: opt_nonempty(&pc.prompts.tool_guidance),
            fallback_guidance: opt_nonempty(&pc.prompts.fallback_guidance),
            json_retry_message: opt_nonempty(&pc.prompts.json_retry_message),
            tool_result_instruction: opt_nonempty(&pc.prompts.tool_result_instruction),
            agent_instructions: opt_nonempty(&pc.prompts.agent_instructions),
            ui_instructions: opt_nonempty(&pc.prompts.ui_instructions),
            language_instructions: opt_nonempty(&pc.prompts.language_instructions),
            agent_max_steps_error: opt_nonempty(&pc.prompts.agent_max_steps_error),
            no_tools_guidance: opt_nonempty(&pc.prompts.no_tools_guidance),
        },
    }
}

fn opt_nonempty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Convert AppConfig to Postcard config
fn convert_to_postcard_config(config: &super::AppConfig) -> postcard_config::PostcardAppConfig {
    postcard_config::PostcardAppConfig {
        server: postcard_config::PostcardServerConfig {
            bind: config.rest_server.bind.clone(),
            cors_origins: config.rest_server.cors_origins.clone(),
            docs: config
                .rest_server
                .docs
                .iter()
                .map(|d| postcard_config::DocServerConfig {
                    url: d.url.clone(),
                    description: d.description.clone(),
                })
                .collect(),
        },
        providers: config
            .providers
            .iter()
            .map(|p| postcard_config::ProviderConfig {
                id: p.id.clone(),
                provider_type: p.provider_type.clone(),
                endpoint: p.endpoint.clone(),
                api_key: p.api_key.clone().unwrap_or_default(),
                models: p
                    .models
                    .iter()
                    .map(|m| postcard_config::ModelInfo {
                        name: m.name.clone(),
                        display_name: m.display_name.clone().unwrap_or_default(),
                    })
                    .collect(),
            })
            .collect(),
        model: postcard_config::ModelConfig {
            default_provider: config.default_provider.clone(),
            model: config.model.clone(),
        },
        prompts: postcard_config::PromptsConfig {
            template: config.prompts.template.clone().unwrap_or_default(),
            tool_guidance: config.prompts.tool_guidance.clone().unwrap_or_default(),
            fallback_guidance: config.prompts.fallback_guidance.clone().unwrap_or_default(),
            json_retry_message: config
                .prompts
                .json_retry_message
                .clone()
                .unwrap_or_default(),
            tool_result_instruction: config
                .prompts
                .tool_result_instruction
                .clone()
                .unwrap_or_default(),
            agent_instructions: config
                .prompts
                .agent_instructions
                .clone()
                .unwrap_or_default(),
            ui_instructions: config.prompts.ui_instructions.clone().unwrap_or_default(),
            language_instructions: config
                .prompts
                .language_instructions
                .clone()
                .unwrap_or_default(),
            agent_max_steps_error: config
                .prompts
                .agent_max_steps_error
                .clone()
                .unwrap_or_default(),
            no_tools_guidance: config.prompts.no_tools_guidance.clone().unwrap_or_default(),
        },
        agent: postcard_config::AgentConfig::default(),
        custom: std::collections::HashMap::new(),
    }
}

/// Initialize default configuration
pub fn init_default_config() -> Result<super::AppConfig, ConfigError> {
    let logger = ConfigLogger::new("config");
    logger.info("Initializing default configuration");

    let config = super::AppConfig::default();
    save_config(&config, None)?;

    logger.info("Default configuration created");
    Ok(config)
}

/// Check if configuration exists
pub fn config_exists() -> bool {
    Path::new(postcard_config::CONFIG_PATH).exists()
}
