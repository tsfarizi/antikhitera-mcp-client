#[test]
#[ignore = "Requires Ollama server running"]
fn test_ollama_provider_integration() {
    // Check if Ollama is available
    require_provider!("ollama");
    
    // If we reach here, Ollama is running
    println!("âœ… Ollama server available, running test...");
    
    // Your test logic here (e.g., test Ollama provider)
    assert!(ollama_available());
}

/// Example test that requires Gemini API key
