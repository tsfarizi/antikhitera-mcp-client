use antikythera_cli::config::{AppConfig, ModelConfig, ProviderConfig};
use antikythera_cli::domain::entities::ProviderType;
use antikythera_cli::infrastructure::config::build_active_provider_config;
#[allow(deprecated)]
use antikythera_cli::infrastructure::config::create_provider_config;

fn sample_config() -> AppConfig {
    AppConfig {
        providers: vec![ProviderConfig {
            id: "ollama-local".to_string(),
            provider_type: "ollama".to_string(),
            endpoint: "http://127.0.0.1:11434".to_string(),
            api_key: String::new(),
            models: Vec::new(),
        }],
        model: ModelConfig {
            default_provider: "ollama-local".to_string(),
            model: "llama3".to_string(),
        },
        ..AppConfig::default()
    }
}

fn openai_config() -> AppConfig {
    AppConfig {
        providers: vec![ProviderConfig {
            id: "openai-gpt".to_string(),
            provider_type: "openai".to_string(),
            endpoint: "https://api.openai.com".to_string(),
            api_key: "sk-test-key".to_string(),
            models: Vec::new(),
        }],
        model: ModelConfig {
            default_provider: "openai-gpt".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        ..AppConfig::default()
    }
}

#[test]
fn build_active_provider_config_maps_ollama_provider() {
    let config = sample_config();
    let provider = build_active_provider_config(&config).expect("provider should map");
    assert_eq!(provider.id, "ollama-local");
    assert_eq!(provider.model, "llama3");
    assert!(provider.api_key.is_none());
}

#[test]
fn build_active_provider_config_maps_openai_provider() {
    let config = openai_config();
    let provider = build_active_provider_config(&config).expect("openai should map");
    assert_eq!(provider.id, "openai-gpt");
    assert_eq!(provider.provider_type, ProviderType::OpenAi);
    assert_eq!(provider.api_key, Some("sk-test-key".to_string()));
}

#[test]
fn build_active_provider_config_rejects_missing_provider() {
    let mut config = sample_config();
    config.model.default_provider = "nonexistent".to_string();
    assert!(build_active_provider_config(&config).is_err());
}

#[test]
fn alias_functions_delegate_to_new_names() {
    let config = sample_config();
    let p1 = build_active_provider_config(&config).expect("p1");
    #[allow(deprecated)]
    let p2 = create_provider_config(&config).expect("p2");
    assert_eq!(p1.id, p2.id);
}
