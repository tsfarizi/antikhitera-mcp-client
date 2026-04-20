#[test]
#[ignore = "Requires config, Ollama, and API keys"]
fn test_full_integration() {
    // Check all prerequisites
    require_all!(
        config_available(),
        provider_env_available("ollama"),
        env_var_exists("GEMINI_API_KEY")
    );
    
    // If we reach here, everything is ready
    println!("âœ… All prerequisites met, running full integration test...");
    
    // Your full integration test logic here
}

/// Test to check test environment setup

#[test]
fn test_environment_check() {
    check_environment();
    
    // This test always passes - it's just for checking environment
    assert!(true, "Environment check completed");
}
