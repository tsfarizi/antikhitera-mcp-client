#[test]
fn test_config_with_custom_values() {
    let mut config = AppConfig::default();
    config.agent.max_steps = 20;
    config.agent.verbose = true;

    let binary = config_to_postcard(&config).expect("Failed to serialize");
    let loaded = config_from_postcard(&binary).expect("Failed to deserialize");

    assert_eq!(loaded.agent.max_steps, 20);
    assert_eq!(loaded.agent.verbose, true);
}

