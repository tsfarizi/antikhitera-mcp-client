#[test]
fn test_config_size() {
    let config = AppConfig::default();
    let binary = config_to_postcard(&config).expect("Failed to serialize");

    // Postcard should produce reasonably small output
    assert!(binary.len() > 0);
    assert!(binary.len() < 10000); // Should be under 10KB for default config
}

