//! Postcard Configuration Tests

use antikythera_sdk::config::*;

#[test]
fn test_config_serialization_roundtrip() {
    let config = AppConfig::default();

    let binary = config_to_postcard(&config).expect("Failed to serialize");
    let loaded = config_from_postcard(&binary).expect("Failed to deserialize");

    assert_eq!(config.model.default_provider, loaded.model.default_provider);
    assert_eq!(config.model.model, loaded.model.model);
    assert_eq!(config.agent.max_steps, loaded.agent.max_steps);
}

#[test]
fn test_config_with_custom_values() {
    let mut config = AppConfig::default();
    config.model.default_provider = "openai".to_string();
    config.model.model = "gpt-4".to_string();
    config.agent.max_steps = 20;

    let binary = config_to_postcard(&config).expect("Failed to serialize");
    let loaded = config_from_postcard(&binary).expect("Failed to deserialize");

    assert_eq!(loaded.model.default_provider, "openai");
    assert_eq!(loaded.model.model, "gpt-4");
    assert_eq!(loaded.agent.max_steps, 20);
}

#[test]
fn test_config_size() {
    let config = AppConfig::default();
    let binary = config_to_postcard(&config).expect("Failed to serialize");

    // Postcard should produce reasonably small output
    assert!(binary.len() > 0);
    assert!(binary.len() < 10000); // Should be under 10KB for default config
}

#[test]
fn test_config_default_values() {
    let config = AppConfig::default();

    // Verify defaults
    assert_eq!(config.model.default_provider, "ollama");
    assert_eq!(config.model.model, "llama3");
    assert_eq!(config.agent.max_steps, 10);
    assert!(!config.agent.verbose);
    assert!(config.agent.auto_execute_tools);
}
