//! Binary Configuration Tests

use antikythera_sdk::config::*;

#[test]
fn test_config_serialization_roundtrip() {
    let config = WasmConfig::default();

    let binary = config_to_binary(&config).expect("Failed to serialize");
    let loaded = config_from_binary(&binary).expect("Failed to deserialize");

    assert_eq!(config.model.default_provider, loaded.model.default_provider);
    assert_eq!(config.model.model, loaded.model.model);
    assert_eq!(config.agent.max_steps, loaded.agent.max_steps);
}

#[test]
fn test_config_with_custom_values() {
    let mut config = WasmConfig::default();
    config.model.default_provider = "openai".to_string();
    config.model.model = "gpt-4".to_string();
    config.agent.max_steps = 20;

    let binary = config_to_binary(&config).expect("Failed to serialize");
    let loaded = config_from_binary(&binary).expect("Failed to deserialize");

    assert_eq!(loaded.model.default_provider, "openai");
    assert_eq!(loaded.model.model, "gpt-4");
    assert_eq!(loaded.agent.max_steps, 20);
}

#[test]
fn test_config_size_breakdown() {
    let config = WasmConfig::default();
    let sizes = config_size_breakdown(&config);

    assert!(sizes.contains_key("client"));
    assert!(sizes.contains_key("model"));
    assert!(sizes.contains_key("prompts"));
    assert!(sizes.contains_key("agent"));

    let total: usize = sizes.values().sum();
    assert!(total > 0);
}

#[test]
fn test_config_summary() {
    let config = WasmConfig::default();
    let summary = config_summary(&config);

    assert!(summary.contains("WASM Configuration"));
    assert!(summary.contains("Binary size"));
}
