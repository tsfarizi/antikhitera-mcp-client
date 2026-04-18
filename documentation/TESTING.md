# Testing

## Overview

Antikythera uses a smart testing system that automatically detects available dependencies and skips tests that can't run. This ensures:

- ✅ Tests always pass when prerequisites are met
- ⚠️ Tests are skipped with clear messages when dependencies are missing
- 📋 Clear instructions on how to enable skipped tests

## Running Tests

### Basic Test Command

```bash
# Run all tests (auto-skips unavailable tests)
cargo test --workspace

# Run only library tests
cargo test --workspace --lib

# Run tests with output visible
cargo test --workspace -- --nocapture
```

### Test Categories

Antikythera has three types of tests:

1. **Unit Tests** - Always run, no external dependencies
2. **Integration Tests** - Require servers/configs (auto-skipped if unavailable)
3. **Environment Check** - Shows what's available

## Environment Check

Run this first to see what's available:

```bash
cargo test --workspace --lib test_environment_check -- --nocapture
```

**Example Output:**

```
🔍 Checking test environment...

✅ Configuration files found
❌ Ollama server not running
   → Run: ollama serve

✅ Gemini API key set
ℹ️  OpenAI API key not set (optional)
ℹ️  Anthropic API key not set (optional)

⚠️  Some prerequisites are missing.

📋 Test Setup Instructions:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Configuration Files:
   cp config.example/client.toml config/client.toml
   cp config.example/model.toml config/model.toml

2. Local Provider (Ollama):
   # Install from https://ollama.ai
   ollama serve
   ollama pull llama3

3. Cloud Providers (Optional):
   export GEMINI_API_KEY=<your-gemini-key>
   export OPENAI_API_KEY=<your-openai-key>
   export ANTHROPIC_API_KEY=<your-anthropic-key>

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Conditional Tests

### How It Works

Tests automatically check for prerequisites before running:

```rust
#[test]
fn test_ollama_provider() {
    // This will skip if Ollama is not running
    require_provider!("ollama");
    
    // Test logic here - only runs if Ollama is available
    assert!(provider_is_ready());
}
```

### Skipped Test Output

When a test is skipped, you'll see:

```
test integration_tests::test_ollama_provider ... ignored
⚠️  SKIPPED: Provider 'ollama' is not available
   To run this test:
   1. Install Ollama: https://ollama.ai
   2. Start Ollama server: ollama serve
   3. Pull a model: ollama pull llama3
```

## Available Macros

### `require_config!()`

Skip test if configuration files are missing.

```rust
#[test]
fn test_config_loading() {
    require_config!();
    // Test logic here
}
```

**Skip Message:**
```
⚠️  SKIPPED: Configuration files not found
   To run this test, please:
   1. Copy config.example/client.toml to config/client.toml
   2. Copy config.example/model.toml to config/model.toml
   3. Update the configuration with your settings
```

---

### `require_server!(host, port)`

Skip test if a server is not running.

```rust
#[test]
fn test_mcp_server() {
    require_server!("127.0.0.1", 8080);
    // Test logic here
}
```

**Skip Message:**
```
⚠️  SKIPPED: Server not available at 127.0.0.1:8080 (required for this test)
```

---

### `require_env!(VAR_NAME)`

Skip test if an environment variable is not set.

```rust
#[test]
fn test_gemini_api() {
    require_env!("GEMINI_API_KEY");
    // Test logic here
}
```

**Skip Message:**
```
⚠️  SKIPPED: Environment variable GEMINI_API_KEY not set
   To run this test, please set: export GEMINI_API_KEY=<value>
```

---

### `require_provider!("provider_name")`

Skip test if a specific provider is not available.

```rust
#[test]
fn test_ollama() {
    require_provider!("ollama");
    // Test logic here
}
```

**Skip Messages by Provider:**

**Ollama:**
```
⚠️  SKIPPED: Provider 'ollama' is not available
   To run this test:
   1. Install Ollama: https://ollama.ai
   2. Start Ollama server: ollama serve
   3. Pull a model: ollama pull llama3
```

**Gemini:**
```
⚠️  SKIPPED: Provider 'gemini' is not available
   To run this test:
   1. Get API key from: https://makersuite.google.com/app/apikey
   2. Set environment variable: export GEMINI_API_KEY=<your-key>
```

**OpenAI:**
```
⚠️  SKIPPED: Provider 'openai' is not available
   To run this test:
   1. Get API key from: https://platform.openai.com/api-keys
   2. Set environment variable: export OPENAI_API_KEY=<your-key>
```

---

### `require_all!(condition1, condition2, ...)`

Skip test if ANY condition is not met.

```rust
#[test]
fn test_full_integration() {
    require_all!(
        config_available(),
        provider_env_available("ollama"),
        env_var_exists("GEMINI_API_KEY")
    );
    // Test logic here
}
```

**Skip Message:**
```
⚠️  SKIPPED: Prerequisite not met
```

## Test Utilities

Available utility functions in `test_utils` module:

```rust
use antikythera_core::test_utils::*;

// Check if TCP port is available
is_port_available("127.0.0.1", 11434)  // bool

// Check if file exists
file_exists("config/client.toml")  // bool

// Check if environment variable is set
env_var_exists("GEMINI_API_KEY")  // bool

// Check if config files are available
config_available()  // bool

// Check if Ollama server is running
ollama_available()  // bool

// Check if provider environment is available
provider_env_available("gemini")  // bool
```

## Writing Your Own Conditional Tests

### Example 1: Test Requiring Config

```rust
#[test]
#[ignore = "Requires configuration"]
fn test_my_config_feature() {
    require_config!();
    
    // Your test logic here
    let config = AppConfig::load(None).unwrap();
    assert!(!config.providers.is_empty());
}
```

### Example 2: Test Requiring Ollama

```rust
#[test]
#[ignore = "Requires Ollama server"]
fn test_ollama_chat() {
    require_provider!("ollama");
    
    // Your Ollama test here
    let response = chat_with_ollama("Hello");
    assert!(!response.is_empty());
}
```

### Example 3: Test Requiring Multiple Providers

```rust
#[test]
#[ignore = "Requires multiple providers"]
fn test_multi_provider_failover() {
    require_all!(
        provider_env_available("ollama"),
        provider_env_available("gemini")
    );
    
    // Your multi-provider test here
    test_failover();
}
```

## Ignored Tests

Some tests are marked with `#[ignore]` and won't run by default. To run them:

```bash
# Run ignored tests
cargo test --workspace -- --ignored

# Run ignored tests with output
cargo test --workspace -- --ignored --nocapture

# Run specific ignored test
cargo test --workspace --lib test_full_integration -- --ignored --nocapture
```

## CI/CD Integration

For CI environments, you can:

1. **Set environment variables** in your CI config
2. **Start servers** before running tests
3. **Use test filters** to run only available tests

### GitHub Actions Example

```yaml
- name: Setup Ollama
  run: |
    curl -fsSL https://ollama.ai/install.sh | sh
    ollama serve &
    ollama pull llama3

- name: Set API Keys
  run: |
    echo "GEMINI_API_KEY=${{ secrets.GEMINI_API_KEY }}" >> $GITHUB_ENV

- name: Run Tests
  run: cargo test --workspace --lib
```

## Troubleshooting

### Tests Being Skipped Unexpectedly

1. **Check environment:**
   ```bash
   cargo test --workspace --lib test_environment_check -- --nocapture
   ```

2. **Verify config files:**
   ```bash
   ls -la config/
   ```

3. **Check server status:**
   ```bash
   netstat -an | grep 11434  # Ollama
   netstat -an | grep 8080   # Custom server
   ```

### Force Running Tests

To force a test to run even if prerequisites might not be met:

```rust
#[test]
fn test_force_run() {
    // Don't use require_* macros
    // Test will run and fail if prerequisites not met
    let config = AppConfig::load(None);
    assert!(config.is_ok(), "Config should be available");
}
```

## Best Practices

1. **Always use `#[ignore]`** for tests requiring external dependencies
2. **Provide clear skip messages** with setup instructions
3. **Use specific require macros** instead of manual checks
4. **Test environment check** regularly to ensure setup is correct
5. **Document prerequisites** in test function names

## Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Test Commands](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Conditional Compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)

---

*Last Updated: 2026-04-01*  
*Version: 0.8.0*
