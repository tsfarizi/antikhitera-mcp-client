#[test]
#[ignore = "Requires GEMINI_API_KEY environment variable"]
fn test_gemini_provider_integration() {
    // Check if Gemini API key is available
    require_env!("GEMINI_API_KEY");
    require_provider!("gemini");
    
    // If we reach here, API key is set
    println!("âœ… Gemini API key available, running test...");
    
    // Your test logic here (e.g., test Gemini provider)
    assert!(env_var_exists("GEMINI_API_KEY"));
}

/// Example test that requires multiple providers

#[test]
#[ignore = "Requires at least one provider configured"]
fn test_multi_provider_availability() {
    // Check if at least one provider is available
    let ollama_ready = provider_env_available("ollama");
    let gemini_ready = provider_env_available("gemini");
    let openai_ready = provider_env_available("openai");
    
    if !ollama_ready && !gemini_ready && !openai_ready {
        println!("âš ï¸  SKIPPED: No providers available");
        println!("   Available providers:");
        println!("   - Ollama: {} (port 11434)", if ollama_ready { "âœ“" } else { "âœ—" });
        println!("   - Gemini: {} (GEMINI_API_KEY)", if gemini_ready { "âœ“" } else { "âœ—" });
        println!("   - OpenAI: {} (OPENAI_API_KEY)", if openai_ready { "âœ“" } else { "âœ—" });
        println!();
        println!("   To enable providers:");
        if !ollama_ready {
            println!("   - Ollama: Install from https://ollama.ai and run 'ollama serve'");
        }
        if !gemini_ready {
            println!("   - Gemini: Get key from https://makersuite.google.com/app/apikey");
            println!("             Then: export GEMINI_API_KEY=<your-key>");
        }
        if !openai_ready {
            println!("   - OpenAI: Get key from https://platform.openai.com/api-keys");
            println!("             Then: export OPENAI_API_KEY=<your-key>");
        }
        return;
    }
    
    println!("âœ… Providers available:");
    if ollama_ready { println!("   - Ollama âœ“"); }
    if gemini_ready { println!("   - Gemini âœ“"); }
    if openai_ready { println!("   - OpenAI âœ“"); }
    
    // Your multi-provider test logic here
}

/// Example test that requires a custom server
