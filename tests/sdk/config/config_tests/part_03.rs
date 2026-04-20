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

