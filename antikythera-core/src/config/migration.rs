//! Migration Tool: TOML → Postcard
//!
//! Migrates existing TOML-based configuration to unified Postcard format.

use super::postcard_config;
use super::app;
use std::path::Path;

/// Migrate from TOML config files to Postcard
pub fn migrate_toml_to_postcard() -> Result<postcard_config::AppConfig, String> {
    // Load old config
    let old_config = app::AppConfig::load(Some(Path::new(postcard_config::CONFIG_PATH)))
        .map_err(|e| format!("Failed to load old config: {}", e))?;

    // Convert to new format
    let new_config = postcard_config::AppConfig {
        server: postcard_config::ServerConfig {
            bind: old_config.rest_server.bind.clone(),
            cors_origins: old_config.rest_server.cors_origins.clone(),
            docs: old_config.rest_server.docs.iter().map(|d| postcard_config::DocServerConfig {
                url: d.url.clone(),
                description: d.description.clone(),
            }).collect(),
        },
        providers: old_config.providers.iter().map(|p| postcard_config::ProviderConfig {
            id: p.id.clone(),
            provider_type: p.provider_type.clone(),
            endpoint: p.endpoint.clone(),
            api_key: p.api_key.clone().unwrap_or_default(),
            models: p.models.iter().map(|m| postcard_config::ModelInfo {
                name: m.name.clone(),
                display_name: m.display_name.clone().unwrap_or_else(|| m.name.clone()),
            }).collect(),
        }).collect(),
        model: postcard_config::ModelConfig {
            default_provider: old_config.default_provider.clone(),
            model: old_config.model.clone(),
        },
        prompts: postcard_config::PromptsConfig {
            template: old_config.prompts.template.clone().unwrap_or_default(),
            tool_guidance: old_config.prompts.tool_guidance.clone().unwrap_or_default(),
            fallback_guidance: old_config.prompts.fallback_guidance.clone().unwrap_or_default(),
            json_retry_message: old_config.prompts.json_retry_message.clone().unwrap_or_default(),
            tool_result_instruction: old_config.prompts.tool_result_instruction.clone().unwrap_or_default(),
            agent_instructions: old_config.prompts.agent_instructions.clone().unwrap_or_default(),
            ui_instructions: old_config.prompts.ui_instructions.clone().unwrap_or_default(),
            language_instructions: old_config.prompts.language_instructions.clone().unwrap_or_default(),
            agent_max_steps_error: old_config.prompts.agent_max_steps_error.clone().unwrap_or_default(),
            no_tools_guidance: old_config.prompts.no_tools_guidance.clone().unwrap_or_default(),
        },
        agent: postcard_config::AgentConfig::default(),
        custom: std::collections::HashMap::new(),
    };

    // Save as Postcard
    postcard_config::save_config(&new_config, None)?;

    Ok(new_config)
}

/// Check if migration is needed
pub fn needs_migration() -> bool {
    // Check if Postcard config doesn't exist yet
    let postcard_path = Path::new(postcard_config::CONFIG_PATH);
    !postcard_path.exists()
}

/// Get migration status
pub fn migration_status() -> String {
    if needs_migration() {
        "Migration needed: Postcard config not yet created".to_string()
    } else if postcard_config::config_exists() {
        "Postcard config is up to date".to_string()
    } else {
        "No configuration found. Run 'init' to create default config.".to_string()
    }
}
