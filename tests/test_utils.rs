//! Test Utilities for Conditional Test Execution
//!
//! This module provides utilities to check if test prerequisites are available
//! (servers, configurations, etc.) and automatically skip tests if they're not.
//!
//! ## Usage
//!
//! ```ignore
//! use tests::test_utils::*;
//!
//! #[test]
//! fn my_test() {
//!     require_config!();
//!     require_provider!("ollama");
//!     // Your test logic here
//! }
//! ```

use std::env;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// Check if a TCP port is available (server is running)
pub fn is_port_available(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    if let Ok(socket_addr) = SocketAddr::from_str(&addr) {
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)).is_ok()
    } else {
        false
    }
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Check if an environment variable is set
pub fn env_var_exists(name: &str) -> bool {
    env::var(name).is_ok()
}

/// Check if config file exists in standard locations
pub fn config_available() -> bool {
    let config_paths = [
        "config/client.toml",
        "config/model.toml",
        "./config/client.toml",
        "./config/model.toml",
    ];
    
    config_paths.iter().any(|path| file_exists(path))
}

/// Check if Ollama server is running (default port 11434)
pub fn ollama_available() -> bool {
    is_port_available("127.0.0.1", 11434)
}

/// Check if specific environment variables for providers are set
pub fn provider_env_available(provider: &str) -> bool {
    match provider {
        "gemini" => env_var_exists("GEMINI_API_KEY"),
        "openai" => env_var_exists("OPENAI_API_KEY"),
        "anthropic" => env_var_exists("ANTHROPIC_API_KEY"),
        "ollama" => ollama_available(),
        _ => false,
    }
}

/// Helper macro to skip test if condition is not met
#[macro_export]
macro_rules! skip_test_if {
    ($condition:expr, $reason:expr) => {
        if $condition {
            println!("⚠️  SKIPPED: {}", $reason);
            return;
        }
    };
}

/// Helper macro to skip test if server is not available
#[macro_export]
macro_rules! require_server {
    ($host:expr, $port:expr) => {
        if !is_port_available($host, $port) {
            println!("⚠️  SKIPPED: Server not available at {}:{} (required for this test)", $host, $port);
            return;
        }
    };
}

/// Helper macro to skip test if config is not available
#[macro_export]
macro_rules! require_config {
    () => {
        if !config_available() {
            println!("⚠️  SKIPPED: Configuration files not found (config/client.toml or config/model.toml)");
            println!("   To run this test, please:");
            println!("   1. Copy config.example/client.toml to config/client.toml");
            println!("   2. Copy config.example/model.toml to config/model.toml");
            println!("   3. Update the configuration with your settings");
            return;
        }
    };
}

/// Helper macro to skip test if environment variable is not set
#[macro_export]
macro_rules! require_env {
    ($var:expr) => {
        if !env_var_exists($var) {
            println!("⚠️  SKIPPED: Environment variable {} not set", $var);
            println!("   To run this test, please set: export {}=<value>", $var);
            return;
        }
    };
}

/// Helper macro to skip test if provider is not available
#[macro_export]
macro_rules! require_provider {
    ($provider:expr) => {
        if !provider_env_available($provider) {
            println!("⚠️  SKIPPED: Provider '{}' is not available", $provider);
            match $provider {
                "ollama" => {
                    println!("   To run this test:");
                    println!("   1. Install Ollama: https://ollama.ai");
                    println!("   2. Start Ollama server: ollama serve");
                    println!("   3. Pull a model: ollama pull llama3");
                }
                "gemini" => {
                    println!("   To run this test:");
                    println!("   1. Get API key from: https://makersuite.google.com/app/apikey");
                    println!("   2. Set environment variable: export GEMINI_API_KEY=<your-key>");
                }
                "openai" => {
                    println!("   To run this test:");
                    println!("   1. Get API key from: https://platform.openai.com/api-keys");
                    println!("   2. Set environment variable: export OPENAI_API_KEY=<your-key>");
                }
                "anthropic" => {
                    println!("   To run this test:");
                    println!("   1. Get API key from: https://console.anthropic.com/settings/keys");
                    println!("   2. Set environment variable: export ANTHROPIC_API_KEY=<your-key>");
                }
                _ => {
                    println!("   Provider configuration not found");
                }
            }
            return;
        }
    };
}

/// Helper macro to skip test if all prerequisites are not met
#[macro_export]
macro_rules! require_all {
    ($($condition:expr),+ $(,)?) => {
        $(
            if !$condition {
                println!("⚠️  SKIPPED: Prerequisite not met");
                return;
            }
        )+
    };
}

/// Helper function to print test setup instructions
pub fn print_setup_instructions() {
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

/// Check test environment and print status
pub fn check_environment() {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_check_ollama() {
        // This test will be skipped if Ollama is not running
        if !ollama_available() {
            println!("⚠️  SKIPPED: Ollama server not running on port 11434");
            println!("   Start Ollama with: ollama serve");
            return;
        }
        assert!(is_port_available("127.0.0.1", 11434));
    }

    #[test]
    fn test_config_check() {
        // This test will be skipped if config files don't exist
        if !config_available() {
            println!("⚠️  SKIPPED: Configuration files not found");
            println!("   Create config/client.toml and config/model.toml");
            return;
        }
        assert!(config_available());
    }

    #[test]
    fn test_env_var_check() {
        // Example: Check if any provider API key is set
        let has_any_key = provider_env_available("ollama")
            || provider_env_available("gemini")
            || provider_env_available("openai");
        
        if !has_any_key {
            println!("⚠️  SKIPPED: No provider API keys found");
            println!("   Set GEMINI_API_KEY, OPENAI_API_KEY, or run Ollama locally");
            return;
        }
        assert!(has_any_key);
    }
}
