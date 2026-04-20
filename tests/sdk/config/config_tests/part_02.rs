#[test]
fn test_config_serialization_roundtrip() {
    let config = AppConfig::default();

    let binary = config_to_postcard(&config).expect("Failed to serialize");
    let loaded = config_from_postcard(&binary).expect("Failed to deserialize");

    assert_eq!(config.model.default_provider, loaded.model.default_provider);
    assert_eq!(config.model.model, loaded.model.model);
    assert_eq!(config.agent.max_steps, loaded.agent.max_steps);
}

