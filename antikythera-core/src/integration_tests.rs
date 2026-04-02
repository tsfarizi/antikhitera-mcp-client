//! Integration Tests with Conditional Execution
//!
//! These tests automatically skip if prerequisites (servers, configs, API keys)
//! are not available. Each skipped test provides clear instructions on how to
//! set up the required dependencies.

use crate::test_utils::*;

/// Example test that requires configuration files
#[test]
#[ignore = "Requires configuration files"]
fn test_config_loading_integration() {
    // This test will check if config is available and skip if not
    require_config!();
    
    // If we reach here, config is available
    println!("✅ Configuration files found, running test...");
    
    // Your test logic here
    assert!(config_available());
}

/// Example test that requires Ollama server
#[test]
#[ignore = "Requires Ollama server running"]
fn test_ollama_provider_integration() {
    // Check if Ollama is available
    require_provider!("ollama");
    
    // If we reach here, Ollama is running
    println!("✅ Ollama server available, running test...");
    
    // Your test logic here (e.g., test Ollama provider)
    assert!(ollama_available());
}

/// Example test that requires Gemini API key
#[test]
#[ignore = "Requires GEMINI_API_KEY environment variable"]
fn test_gemini_provider_integration() {
    // Check if Gemini API key is available
    require_env!("GEMINI_API_KEY");
    require_provider!("gemini");
    
    // If we reach here, API key is set
    println!("✅ Gemini API key available, running test...");
    
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
        println!("⚠️  SKIPPED: No providers available");
        println!("   Available providers:");
        println!("   - Ollama: {} (port 11434)", if ollama_ready { "✓" } else { "✗" });
        println!("   - Gemini: {} (GEMINI_API_KEY)", if gemini_ready { "✓" } else { "✗" });
        println!("   - OpenAI: {} (OPENAI_API_KEY)", if openai_ready { "✓" } else { "✗" });
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
    
    println!("✅ Providers available:");
    if ollama_ready { println!("   - Ollama ✓"); }
    if gemini_ready { println!("   - Gemini ✓"); }
    if openai_ready { println!("   - OpenAI ✓"); }
    
    // Your multi-provider test logic here
}

/// Example test that requires a custom server
#[test]
#[ignore = "Requires custom MCP server running"]
fn test_custom_mcp_server() {
    // Check if custom server is running on port 8080
    require_server!("127.0.0.1", 8080);
    
    // If we reach here, server is running
    println!("✅ MCP server available on port 8080, running test...");
    
    // Your test logic here (e.g., test MCP server integration)
    assert!(is_port_available("127.0.0.1", 8080));
}

/// Example test that requires all prerequisites
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
    println!("✅ All prerequisites met, running full integration test...");
    
    // Your full integration test logic here
}

/// Helper function to print test setup instructions
fn print_setup_instructions() {
    println!("\n📋 Test Setup Instructions:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("1. Configuration Files:");
    println!("   cp config.example/client.toml config/client.toml");
    println!("   cp config.example/model.toml config/model.toml");
    println!();
    println!("2. Local Provider (Ollama):");
    println!("   # Install from https://ollama.ai");
    println!("   ollama serve");
    println!("   ollama pull llama3");
    println!();
    println!("3. Cloud Providers (Optional):");
    println!("   export GEMINI_API_KEY=<your-gemini-key>");
    println!("   export OPENAI_API_KEY=<your-openai-key>");
    println!("   export ANTHROPIC_API_KEY=<your-anthropic-key>");
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// Test to check test environment setup
#[test]
fn test_environment_check() {
    println!("\n🔍 Checking test environment...\n");
    
    let mut all_good = true;
    
    // Check config
    if config_available() {
        println!("✅ Configuration files found");
    } else {
        println!("❌ Configuration files not found");
        println!("   → Copy config.example/*.toml to config/\n");
        all_good = false;
    }
    
    // Check Ollama
    if ollama_available() {
        println!("✅ Ollama server running (port 11434)");
    } else {
        println!("❌ Ollama server not running");
        println!("   → Run: ollama serve\n");
        all_good = false;
    }
    
    // Check API keys
    let api_keys = [
        ("GEMINI_API_KEY", "Gemini"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("ANTHROPIC_API_KEY", "Anthropic"),
    ];
    
    for (key, name) in api_keys.iter() {
        if env_var_exists(key) {
            println!("✅ {} API key set", name);
        } else {
            println!("ℹ️  {} API key not set (optional)", name);
        }
    }
    
    println!();
    
    if all_good {
        println!("🎉 Test environment is ready!\n");
    } else {
        println!("⚠️  Some prerequisites are missing.\n");
        print_setup_instructions();
    }
    
    // This test always passes - it's just for checking environment
    assert!(true, "Environment check completed");
}
