# 📁 Centralized Test Structure

## Overview

All test code has been centralized in the `tests/` folder at the workspace root. This follows Rust best practices for integration testing.

## Structure

```
antikythera-mcp-framework/
├── tests/
│   ├── test_utils.rs           # Test utilities & conditional macros
│   ├── integration_tests.rs    # Integration tests with auto-skip
│   ├── config_tests.rs         # Config tests
│   ├── provider_tests.rs       # Provider tests
│   ├── server_tests.rs         # Server tests
│   ├── tooling_tests.rs        # Tooling tests
│   ├── transport_tests.rs      # Transport tests
│   ├── tui_tests.rs            # TUI tests
│   ├── wizard_tests.rs         # Wizard tests
│   ├── concurrency_tests.rs    # Concurrency tests
│   ├── serialization_tests.rs  # Serialization tests
│   ├── integration_runner.rs   # Integration runner
│   │
│   ├── config/                 # Config test modules
│   ├── provider/               # Provider test modules
│   ├── server/                 # Server test modules
│   ├── serialization/          # Serialization test modules
│   ├── tui/                    # TUI test modules
│   └── ...                     # Other test modules
│
└── src/
    ├── antikythera-core/       # Core library (NO TEST CODE)
    ├── antikythera-sdk/        # SDK (NO TEST CODE)
    └── antikythera-cli/        # CLI (NO TEST CODE)
```

## Key Changes

### ✅ Before (Scattered Tests)
```
src/
├── lib.rs
├── test_utils.rs          ❌ Test code in src/
├── integration_tests.rs   ❌ Test code in src/
└── application/
    └── agent/
        └── memory.rs
            └── mod tests  ❌ Inline tests
```

### ✅ After (Centralized Tests)
```
tests/
├── test_utils.rs          ✅ Centralized utilities
├── integration_tests.rs   ✅ Centralized integration tests
└── ...                    ✅ All other tests

src/
└── antikythera-core/      ✅ Production code only
```

## Running Tests

### All Tests
```bash
# Run all tests (auto-skips unavailable tests)
cargo test --workspace

# Run with output visible
cargo test --workspace -- --nocapture
```

### Specific Test Files
```bash
# Run integration tests
cargo test --workspace --test integration_tests

# Run test utilities tests
cargo test --workspace --test test_utils

# Run config tests
cargo test --workspace --test config_tests
```

### Environment Check
```bash
# Check test environment
cargo test --workspace --test integration_tests test_environment_check -- --nocapture
```

## Test Utilities

Located in `tests/test_utils.rs`:

### Functions
```rust
use test_utils::*;

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

### Macros
```rust
// Skip if config missing
require_config!();

// Skip if server not running
require_server!("127.0.0.1", 8080);

// Skip if env var not set
require_env!("GEMINI_API_KEY");

// Skip if provider not available
require_provider!("ollama");

// Skip if any condition not met
require_all!(
    config_available(),
    provider_env_available("ollama")
);
```

## Writing Tests

### Example Integration Test

```rust
// tests/my_integration_test.rs

mod test_utils;
use test_utils::*;

#[test]
#[ignore = "Requires Ollama server"]
fn test_ollama_chat() {
    require_provider!("ollama");
    
    println!("✅ Ollama available, running test...");
    
    // Your test logic here
    assert!(ollama_available());
}
```

### Example Unit Test (in src/)

```rust
// src/antikythera-core/src/application/agent/memory.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_load() {
        // Unit tests for internal logic
        // These stay in src/ as they test implementation details
        let temp_dir = tempdir().unwrap();
        let mut provider = FilesystemMemory::new(temp_dir.path().to_path_buf());
        
        // Test logic...
    }
}
```

## Test Categories

### 1. Unit Tests (in `src/`)
- **Location**: Inside source files with `#[cfg(test)]`
- **Purpose**: Test internal implementation details
- **Dependencies**: None (mocked)
- **Example**: `memory.rs` tests for FilesystemMemory

### 2. Integration Tests (in `tests/`)
- **Location**: `tests/` folder
- **Purpose**: Test public API and external integrations
- **Dependencies**: Servers, configs, API keys (auto-skipped if unavailable)
- **Example**: `integration_tests.rs`

### 3. Conditional Tests (in `tests/`)
- **Location**: `tests/integration_tests.rs`
- **Purpose**: Tests that require specific setup
- **Dependencies**: Auto-detected and skipped if unavailable
- **Example**: Provider availability tests

## Benefits

### ✅ Centralized Test Code
- All test utilities in one place
- Easy to find and maintain tests
- Clear separation of test vs production code

### ✅ Auto-Skip Mechanism
- Tests automatically skip if dependencies unavailable
- Clear messages explaining WHY test was skipped
- Instructions on HOW to enable skipped tests

### ✅ Better Organization
- Production code in `src/`
- Test code in `tests/`
- No mixing of test and production logic

### ✅ Easier CI/CD
- Simple to run all tests: `cargo test`
- Easy to filter by test type
- Clear test output and reporting

## Migration Guide

### Moving Tests from `src/` to `tests/`

1. **Create test file in `tests/`:**
   ```rust
   // tests/my_feature_tests.rs
   mod test_utils;
   use test_utils::*;
   
   #[test]
   fn test_my_feature() {
       require_config!();
       // Test logic
   }
   ```

2. **Remove from `src/`:**
   ```rust
   // src/lib.rs
   // Remove: #[cfg(test)] mod tests;
   // Remove: pub mod test_utils;
   ```

3. **Update imports:**
   ```rust
   // Before (in src/)
   use crate::test_utils::*;
   
   // After (in tests/)
   mod test_utils;
   use test_utils::*;
   ```

## Best Practices

1. **Keep unit tests in `src/`** - For testing private implementation details
2. **Put integration tests in `tests/`** - For testing public API
3. **Use conditional macros** - Auto-skip tests with missing dependencies
4. **Provide clear skip messages** - Help developers enable skipped tests
5. **Group related tests** - Use subfolders in `tests/` for organization

## Troubleshooting

### Test Not Found
```bash
# List all available tests
cargo test --workspace -- --list

# Run specific test
cargo test --workspace --test <test_file> <test_name>
```

### Import Errors
```rust
// In tests/, always include:
mod test_utils;
use test_utils::*;
```

### Macro Not Found
```rust
// Ensure test_utils module is declared first
mod test_utils;
use test_utils::*;  // Now macros are available
```

## Additional Resources

- [Rust Integration Testing](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [TESTING_GUIDE.md](./TESTING_GUIDE.md) - Complete testing guide

---

*Last Updated: 2026-04-01*  
*Version: 0.8.0*
