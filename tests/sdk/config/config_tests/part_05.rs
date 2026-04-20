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
