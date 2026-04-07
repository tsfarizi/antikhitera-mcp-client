//! Configuration loader - Postcard-only
//!
//! All configuration is stored as a single Postcard binary file (`app.pc`).

use super::app::{PromptsConfig, RestServerConfig};
use super::error::ConfigError;
use super::provider::ModelProviderConfig;
use super::postcard_config;
use dotenvy::from_filename;
use std::path::Path;
use std::sync::Once;
use tracing::debug;

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

    debug!(path = %config_path.display(), "Loading configuration from Postcard");

    if !config_path.exists() {
        return Err(ConfigError::NotFound {
            path: config_path.to_path_buf(),
        });
    }

    let postcard_config = postcard_config::load_config(Some(config_path))
        .map_err(|e| ConfigError::CacheError(format!("Failed to load Postcard config: {}", e)))?;

    Ok(convert_postcard_to_app_config(&postcard_config))
}

/// Convert Postcard AppConfig to application AppConfig
fn convert_postcard_to_app_config(postcard: &postcard_config::AppConfig) -> super::AppConfig {
    super::AppConfig {
        default_provider: postcard.model.default_provider.clone(),
        model: postcard.model.model.clone(),
        system_prompt: None,
        tools: Vec::new(),
        servers: Vec::new(),
        providers: postcard.providers.iter().map(|p| ModelProviderConfig {
            id: p.id.clone(),
            provider_type: p.provider_type.clone(),
            endpoint: p.endpoint.clone(),
            api_key: if p.api_key.is_empty() { None } else { Some(p.api_key.clone()) },
            api_path: None,
            models: p.models.iter().map(|m| crate::config::provider::ModelInfo {
                name: m.name.clone(),
                display_name: if m.display_name.is_empty() { None } else { Some(m.display_name.clone()) },
            }).collect(),
        }).collect(),
        rest_server: RestServerConfig {
            bind: postcard.server.bind.clone(),
            cors_origins: postcard.server.cors_origins.clone(),
            docs: postcard.server.docs.iter().map(|d| crate::config::app::DocServerConfig {
                url: d.url.clone(),
                description: d.description.clone(),
            }).collect(),
        },
        prompts: PromptsConfig {
            template: opt_nonempty(&postcard.prompts.template),
            tool_guidance: opt_nonempty(&postcard.prompts.tool_guidance),
            fallback_guidance: opt_nonempty(&postcard.prompts.fallback_guidance),
            json_retry_message: opt_nonempty(&postcard.prompts.json_retry_message),
            tool_result_instruction: opt_nonempty(&postcard.prompts.tool_result_instruction),
            agent_instructions: opt_nonempty(&postcard.prompts.agent_instructions),
            ui_instructions: opt_nonempty(&postcard.prompts.ui_instructions),
            language_instructions: opt_nonempty(&postcard.prompts.language_instructions),
            agent_max_steps_error: opt_nonempty(&postcard.prompts.agent_max_steps_error),
            no_tools_guidance: opt_nonempty(&postcard.prompts.no_tools_guidance),
        },
    }
}

fn opt_nonempty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

/// Convert application AppConfig to Postcard AppConfig
pub fn convert_app_to_postcard_config(app: &super::AppConfig) -> postcard_config::AppConfig {
    postcard_config::AppConfig {
        server: postcard_config::ServerConfig {
            bind: app.rest_server.bind.clone(),
            cors_origins: app.rest_server.cors_origins.clone(),
            docs: app.rest_server.docs.iter().map(|d| postcard_config::DocServerConfig {
                url: d.url.clone(),
                description: d.description.clone(),
            }).collect(),
        },
        providers: app.providers.iter().map(|p| postcard_config::ProviderConfig {
            id: p.id.clone(),
            provider_type: p.provider_type.clone(),
            endpoint: p.endpoint.clone(),
            api_key: p.api_key.clone().unwrap_or_default(),
            models: p.models.iter().map(|m| postcard_config::ModelInfo {
                name: m.name.clone(),
                display_name: m.display_name.clone().unwrap_or_default(),
            }).collect(),
        }).collect(),
        model: postcard_config::ModelConfig {
            default_provider: app.default_provider.clone(),
            model: app.model.clone(),
        },
        prompts: postcard_config::PromptsConfig {
            template: app.prompts.template.clone().unwrap_or_default(),
            tool_guidance: app.prompts.tool_guidance.clone().unwrap_or_default(),
            fallback_guidance: app.prompts.fallback_guidance.clone().unwrap_or_default(),
            json_retry_message: app.prompts.json_retry_message.clone().unwrap_or_default(),
            tool_result_instruction: app.prompts.tool_result_instruction.clone().unwrap_or_default(),
            agent_instructions: app.prompts.agent_instructions.clone().unwrap_or_default(),
            ui_instructions: app.prompts.ui_instructions.clone().unwrap_or_default(),
            language_instructions: app.prompts.language_instructions.clone().unwrap_or_default(),
            agent_max_steps_error: app.prompts.agent_max_steps_error.clone().unwrap_or_default(),
            no_tools_guidance: app.prompts.no_tools_guidance.clone().unwrap_or_default(),
        },
        agent: postcard_config::AgentConfig::default(),
        custom: std::collections::HashMap::new(),
    }
}
