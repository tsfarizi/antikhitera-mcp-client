#[test]
#[ignore = "Requires configuration files"]
fn test_config_loading_integration() {
    // This test will check if config is available and skip if not
    require_config!();
    
    // If we reach here, config is available
    println!("âœ… Configuration files found, running test...");
    
    // Your test logic here
    assert!(config_available());
}

/// Example test that requires Ollama server
