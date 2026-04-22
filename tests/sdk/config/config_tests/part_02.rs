#[test]
fn test_config_serialization_roundtrip() {
    let config = AppConfig::default();

    let binary = config_to_postcard(&config).expect("Failed to serialize");
    let loaded = config_from_postcard(&binary).expect("Failed to deserialize");

    assert_eq!(config.agent.max_steps, loaded.agent.max_steps);
    assert_eq!(config.agent.verbose, loaded.agent.verbose);
}

