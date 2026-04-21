use std::sync::Arc;

use crate::CliError;
use crate::CliResult;
use crate::infrastructure::llm::build_provider_from_configs;
use antikythera_core::config::{ModelInfo, ModelProviderConfig};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_core::{AppConfig, ClientConfig, McpClient};

pub fn build_runtime_client(config: &AppConfig) -> CliResult<Arc<McpClient<DynamicModelProvider>>> {
    let provider = build_provider_from_configs(&config.providers)
        .map_err(|error| CliError::Validation(error.user_message()))?;

    let mut client_config =
        ClientConfig::new(config.default_provider.clone(), config.model.clone())
            .with_tools(config.tools.clone())
            .with_servers(config.servers.clone())
            .with_prompts(config.prompts.clone())
            .with_providers(config.providers.clone());

    if let Some(system) = config.system_prompt.clone() {
        client_config = client_config.with_system_prompt(system);
    }

    Ok(Arc::new(McpClient::new(provider, client_config)))
}

pub fn materialize_runtime_config(
    base: &AppConfig,
    provider_override: Option<&str>,
    model_override: Option<&str>,
    provider_endpoint_override: Option<&str>,
    ollama_url: Option<&str>,
    system_override: Option<&str>,
) -> CliResult<AppConfig> {
    let mut config = base.clone();

    if let Some(system) = system_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        config.system_prompt = Some(system.to_string());
    }

    let mut default_provider = provider_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| match config.default_provider.as_str() {
            // Empty, placeholder, or the generic fallback "ollama" all mean
            // "not explicitly configured" — let env detection choose the best
            // provider based on available API keys.
            "" | "local" | "ollama" => None,
            other => Some(other.to_string()),
        })
        .unwrap_or_else(detect_provider_from_env);

    let mut model = model_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| match config.model.as_str() {
            "" | "default" => None,
            other => Some(other.to_string()),
        })
        .unwrap_or_else(|| default_model_for_provider(&default_provider).to_string());

    if config
        .providers
        .iter()
        .all(|provider| provider.id != default_provider)
        && let Some(template) = default_provider_template(&default_provider)
    {
        config.providers.push(template);
    }

    apply_provider_overrides(
        &mut config.providers,
        &default_provider,
        provider_endpoint_override,
        ollama_url,
    );

    let Some(selected_provider) = config
        .providers
        .iter_mut()
        .find(|provider| provider.id == default_provider)
    else {
        let available = config
            .providers
            .iter()
            .map(|provider| provider.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(CliError::Validation(format!(
            "Provider '{}' tidak tersedia. Provider terdaftar: {}",
            default_provider,
            if available.is_empty() {
                "(tidak ada)"
            } else {
                &available
            }
        )));
    };

    if selected_provider
        .models
        .iter()
        .all(|known| known.name != model)
    {
        selected_provider.ensure_model(&model);
    }

    default_provider = selected_provider.id.clone();
    model = model.trim().to_string();

    config.default_provider = default_provider;
    config.model = model;

    Ok(config)
}

fn apply_provider_overrides(
    providers: &mut [ModelProviderConfig],
    selected_provider: &str,
    provider_endpoint_override: Option<&str>,
    ollama_url: Option<&str>,
) {
    for provider in providers.iter_mut() {
        if provider.is_ollama()
            && let Some(ollama_url) = ollama_url.map(str::trim).filter(|value| !value.is_empty())
        {
            provider.endpoint = ollama_url.to_string();
        }

        if let Some(endpoint) = provider_endpoint_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
            && provider.id == selected_provider
        {
            provider.endpoint = endpoint.to_string();
        }
    }
}

/// Detect the best default provider from environment variables.
///
/// Priority:
/// 1. `GEMINI_API_KEY` set and non-empty → `gemini`
/// 2. `OPENAI_API_KEY` set and non-empty → `openai`
/// 3. Fallback → `ollama` (no API key required)
///
/// The caller is responsible for loading `.env` before this function is
/// invoked (e.g. by calling `dotenvy::dotenv()` at process startup).
fn detect_provider_from_env() -> String {
    if std::env::var("GEMINI_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return "gemini".to_string();
    }

    if std::env::var("OPENAI_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return "openai".to_string();
    }

    "ollama".to_string()
}

fn default_model_for_provider(provider_id: &str) -> &'static str {
    match provider_id.to_ascii_lowercase().as_str() {
        "gemini" => "gemini-2.0-flash",
        "openai" => "gpt-4o-mini",
        _ => "llama3.2",
    }
}

fn default_provider_template(provider_id: &str) -> Option<ModelProviderConfig> {
    match provider_id.to_ascii_lowercase().as_str() {
        "gemini" => Some(ModelProviderConfig {
            id: "gemini".to_string(),
            provider_type: "gemini".to_string(),
            endpoint: "https://generativelanguage.googleapis.com".to_string(),
            // Resolve the actual key value from the environment so the runtime
            // uses the real token, not the variable name as a literal string.
            api_key: std::env::var("GEMINI_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            api_path: None,
            models: vec![ModelInfo {
                name: "gemini-2.0-flash".to_string(),
                display_name: Some("Gemini 2.0 Flash".to_string()),
            }],
        }),
        "openai" => Some(ModelProviderConfig {
            id: "openai".to_string(),
            provider_type: "openai".to_string(),
            endpoint: "https://api.openai.com".to_string(),
            api_key: std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            api_path: None,
            models: vec![ModelInfo {
                name: "gpt-4o-mini".to_string(),
                display_name: Some("GPT-4o Mini".to_string()),
            }],
        }),
        "ollama" => Some(ModelProviderConfig {
            id: "ollama".to_string(),
            provider_type: "ollama".to_string(),
            endpoint: "http://127.0.0.1:11434".to_string(),
            api_key: None,
            api_path: None,
            models: vec![ModelInfo {
                name: "llama3.2".to_string(),
                display_name: Some("Llama 3.2".to_string()),
            }],
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn sample_config() -> AppConfig {
        AppConfig {
            default_provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            system_prompt: None,
            tools: Vec::new(),
            servers: Vec::new(),
            providers: vec![default_provider_template("ollama").expect("ollama template")],
            rest_server: Default::default(),
            prompts: Default::default(),
        }
    }

    #[test]
    fn materialize_runtime_config_can_auto_add_gemini_template() {
        let runtime = materialize_runtime_config(
            &sample_config(),
            Some("gemini"),
            Some("gemini-2.0-flash"),
            None,
            None,
            None,
        )
        .expect("runtime config");
        assert_eq!(runtime.default_provider, "gemini");
        assert_eq!(runtime.model, "gemini-2.0-flash");
        assert!(
            runtime
                .providers
                .iter()
                .any(|provider| provider.id == "gemini")
        );
    }

    #[test]
    fn materialize_runtime_config_applies_selected_endpoint_override() {
        let runtime = materialize_runtime_config(
            &sample_config(),
            Some("openai"),
            Some("gpt-4o-mini"),
            Some("https://example-openai-proxy.test"),
            None,
            None,
        )
        .expect("runtime config");
        let provider = runtime
            .providers
            .iter()
            .find(|provider| provider.id == "openai")
            .expect("openai provider present");
        assert_eq!(provider.endpoint, "https://example-openai-proxy.test");
    }

    #[test]
    #[serial]
    fn detect_provider_from_env_falls_back_to_ollama_when_no_keys() {
        // Remove env vars if they happen to be set in the test environment.
        unsafe {
            std::env::remove_var("GEMINI_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }
        assert_eq!(detect_provider_from_env(), "ollama");
    }

    #[test]
    #[serial]
    fn detect_provider_from_env_prefers_gemini_when_key_present() {
        unsafe {
            std::env::remove_var("GEMINI_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::set_var("GEMINI_API_KEY", "test-key");
        }
        let result = detect_provider_from_env();
        unsafe {
            std::env::remove_var("GEMINI_API_KEY");
        }
        assert_eq!(result, "gemini");
    }

    #[test]
    #[serial]
    fn detect_provider_from_env_uses_openai_when_only_openai_key_present() {
        unsafe {
            std::env::remove_var("GEMINI_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::set_var("OPENAI_API_KEY", "test-openai-key");
        }
        let result = detect_provider_from_env();
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        assert_eq!(result, "openai");
    }
}
