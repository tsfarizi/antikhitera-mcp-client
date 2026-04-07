//! Config loading for CLI

use crate::config::CliConfig;
use crate::domain::entities::*;
use std::error::Error;
use std::path::Path;

pub const CLI_CONFIG_PATH: &str = "cli-config.pc";

/// Load CLI config from Postcard file
pub fn load_cli_config(path: Option<&Path>) -> Result<CliConfig, Box<dyn Error + Send + Sync>> {
    let config_path = path.unwrap_or(Path::new(CLI_CONFIG_PATH));

    if !config_path.exists() {
        return Err(format!("Config not found: {}", config_path.display()).into());
    }

    let data = std::fs::read(config_path)?;
    let config = crate::config::config_from_postcard(&data)?;
    Ok(config)
}

/// Create LLM provider from CLI config
pub fn create_llm_provider(config: &CliConfig) -> Result<Box<dyn crate::domain::use_cases::chat_use_case::LlmProvider>, Box<dyn Error + Send + Sync>> {
    let provider = config.providers
        .iter()
        .find(|p| p.id == config.default_provider)
        .ok_or_else(|| format!("Provider '{}' not found", config.default_provider))?;

    match provider.provider_type.to_lowercase().as_str() {
        "gemini" => {
            let api_key = provider.api_key.clone();
            let model = config.model.clone();
            Ok(Box::new(crate::infrastructure::llm::GeminiProvider::new(api_key, model)))
        }
        "ollama" => {
            let model = config.model.clone();
            let endpoint = provider.endpoint.clone();
            let ollama = crate::infrastructure::llm::OllamaProvider::new(model);
            Ok(Box::new(ollama.with_endpoint(endpoint)))
        }
        other => Err(format!("Unsupported provider: {}", other).into()),
    }
}

/// Create provider config from CLI config
pub fn create_provider_config(config: &CliConfig) -> Result<ProviderConfig, Box<dyn Error + Send + Sync>> {
    let provider = config.providers
        .iter()
        .find(|p| p.id == config.default_provider)
        .ok_or_else(|| format!("Provider '{}' not found", config.default_provider))?;

    let provider_type = ProviderType::from_str(&provider.provider_type)
        .ok_or_else(|| format!("Unknown provider type: {}", provider.provider_type))?;

    Ok(ProviderConfig {
        id: provider.id.clone(),
        provider_type,
        endpoint: provider.endpoint.clone(),
        api_key: if provider.api_key.is_empty() { None } else { Some(provider.api_key.clone()) },
        model: config.model.clone(),
    })
}
