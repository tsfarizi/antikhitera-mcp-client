# Security Features

This document describes the comprehensive security features implemented in the Antikythera MCP Framework, including input validation, rate limiting, and secrets management.

## Overview

The security module provides three main components:

1. **Input Validation** - Comprehensive validation and sanitization of user inputs and WASM component data
2. **Rate Limiting** - Configurable rate limiting with multiple time windows and burst allowance
3. **Secrets Management** - Secure storage and rotation of sensitive data with encryption at rest

All security parameters are configurable via FFI, allowing hosts to customize security policies for their specific use cases.

## Architecture

### Core Module (`antikythera-core`)

The security functionality is implemented in the `antikythera-core` crate under the `security` module:

```
antikythera-core/src/security/
├── mod.rs
├── config.rs
├── validation/
│   ├── mod.rs
│   ├── json.rs
│   ├── types.rs
│   └── url.rs
├── rate_limit.rs
└── secrets/
    ├── mod.rs
    ├── crypto.rs
    ├── error.rs
    └── storage.rs
```

### SDK Module (`antikythera-sdk`)

FFI bindings are provided in the `antikythera-sdk` crate under the `security_ffi` module:

```
antikythera-sdk/src/security_ffi/
├── mod.rs
├── helpers.rs
├── validation.rs
├── rate_limit.rs
└── secrets.rs
```

All security FFI functions use the `ffi_handler!` macro defined in `antikythera-sdk/src/ffi_helpers.rs` for consistent error handling and C string memory management.

## Input Validation

### Features

- **Size Validation**: Enforces maximum input size limits
- **Message Length Validation**: Limits message character count
- **URL Validation**: Validates and sanitizes URLs using regex patterns
- **HTML Sanitization**: Removes dangerous HTML elements and attributes
- **JSON Validation**: Validates JSON structure and schema
- **Keyword Blocking**: Blocks inputs containing dangerous keywords
- **Concurrent Call Limits**: Enforces maximum concurrent tool calls

### Error Types

- **`InputValidatorError`**: Enum error type with variants:
  - `InvalidInput` — Input failed validation checks
  - `Rejected` — Input was explicitly rejected by policy
  - `Configuration` — Invalid validator configuration
- **`ValidationError`**: Struct with `field: String` and `message: String` fields describing the specific validation failure
- **`ValidationResult::Sanitized`**: Additional variant of `ValidationResult` returned when input was sanitized (e.g., HTML stripping)

### Configuration

```rust
pub struct ValidationConfig {
    pub max_input_size_bytes: u64,
    pub max_message_length: usize,
    pub max_concurrent_tool_calls: u32,
    pub allowed_url_patterns: Vec<String>,
    pub blocked_url_patterns: Vec<String>,
    pub sanitize_html: bool,
    pub validate_json_schema: bool,
    pub max_json_nesting_depth: u32,
    pub max_json_array_length: u32,
    pub blocked_keywords: Vec<String>,
}
```

### Default Configuration

- **Max Input Size**: 10 MB
- **Max Message Length**: 100,000 characters
- **Max Concurrent Tool Calls**: 10
- **HTML Sanitization**: Enabled
- **JSON Schema Validation**: Enabled
- **Max JSON Nesting Depth**: 10
- **Max JSON Array Length**: 1,000

### Usage

#### Rust API

```rust
use antikythera_core::security::validation::InputValidator;

// Create validator with default config
let validator = InputValidator::from_config()?;

// Validate input
match validator.validate("user input") {
    Ok(_) => println!("Input is valid"),
    Err(errors) => println!("Validation errors: {:?}", errors),
}

// Validate URL
match validator.validate_url("https://api.example.com") {
    ValidationResult::Valid => println!("URL is valid"),
    ValidationResult::Invalid(msg) => println!("Invalid URL: {}", msg),
    _ => {}
}

// Sanitize HTML
let sanitized = validator.sanitize_html("<script>alert('xss')</script>");
```

#### FFI API

```c
// Initialize validator
mcp_security_init_validator();

// Validate input
char* result = mcp_security_validate_input("user input");
// Process result...
mcp_free_string(result);

// Validate URL
char* result = mcp_security_validate_url("https://api.example.com");
// Process result...
mcp_free_string(result);

// Get configuration
char* config = mcp_security_get_validation_config();
// Process config...
mcp_free_string(config);

// Set configuration
char* new_config = "{\"max_input_size_bytes\": 5242880, ...}";
char* result = mcp_security_set_validation_config(new_config);
// Process result...
mcp_free_string(result);
```

## Rate Limiting

### Features

- **Multiple Time Windows**: Per-minute, per-hour, and per-day limits
- **Burst Allowance**: Allows temporary bursts above the limit
- **Session Management**: Tracks usage per session
- **Concurrent Session Limits**: Enforces maximum concurrent sessions
- **Automatic Cleanup**: Removes inactive sessions automatically

### Configuration

```rust
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub max_concurrent_sessions: u32,
    pub window_size_secs: u32,
    pub burst_allowance: u32,
    pub cleanup_interval_secs: u32,
}
```

### Default Configuration

- **Enabled**: Yes
- **Requests Per Minute**: 60
- **Requests Per Hour**: 1,000
- **Requests Per Day**: 10,000
- **Max Concurrent Sessions**: 5
- **Window Size**: 60 seconds
- **Burst Allowance**: 10 requests
- **Cleanup Interval**: 300 seconds

### Usage

#### Rust API

```rust
use antikythera_core::security::rate_limit::RateLimiter;

// Create rate limiter with default config
let limiter = RateLimiter::from_config();

// Check if request is allowed
match limiter.check("session-id") {
    Ok(_) => println!("Request allowed"),
    Err(e) => println!("Rate limit exceeded: {}", e),
}

// Get usage statistics
if let Some(usage) = limiter.get_usage("session-id") {
    println!("Requests per minute: {}", usage.requests_per_minute);
}

// Reset session
limiter.reset_session("session-id");

// Remove session
limiter.remove_session("session-id");
```

#### FFI API

```c
// Initialize rate limiter
mcp_security_init_rate_limiter();

// Check rate limit
char* result = mcp_security_check_rate_limit("session-id");
// Process result...
mcp_free_string(result);

// Get usage
char* usage = mcp_security_get_usage("session-id");
// Process usage...
mcp_free_string(usage);

// Reset session
char* result = mcp_security_reset_session("session-id");
// Process result...
mcp_free_string(result);

// Get configuration
char* config = mcp_security_get_rate_limit_config();
// Process config...
mcp_free_string(config);

// Set configuration
char* new_config = "{\"requests_per_minute\": 100, ...}";
char* result = mcp_security_set_rate_limit_config(new_config);
// Process result...
mcp_free_string(result);
```

## Secrets Management

### Features

- **Secure Storage**: Encrypted storage for sensitive data
- **Secret Rotation**: Automatic or manual secret rotation
- **Versioning**: Track multiple versions of secrets
- **Metadata Tracking**: Track creation, rotation, and expiration
- **Multiple Backends**: Memory or file-based storage
- **Encryption at Rest**: AES-256-GCM encryption

### Configuration

```rust
pub struct SecretsConfig {
    pub enabled: bool,
    pub encrypt_at_rest: bool,
    pub encryption_algorithm: String,
    pub key_derivation_function: String,
    pub key_derivation_iterations: u32,
    pub auto_rotate: bool,
    pub rotation_interval_hours: u32,
    pub max_secret_age_hours: u32,
    pub storage_backend: String,
    pub storage_path: Option<String>,
    pub enable_versioning: bool,
    pub max_versions: u32,
}
```

### Default Configuration

- **Enabled**: Yes
- **Encrypt at Rest**: Yes
- **Encryption Algorithm**: AES-256-GCM
- **Key Derivation Function**: Argon2
- **Key Derivation Iterations**: 100,000
- **Auto Rotate**: No
- **Rotation Interval**: 720 hours (30 days)
- **Max Secret Age**: 2,160 hours (90 days)
- **Storage Backend**: Memory
- **Versioning**: Enabled
- **Max Versions**: 5

### Usage

#### Rust API

```rust
use antikythera_core::security::secrets::SecretManager;

// Create secret manager with default config
let manager = SecretManager::from_config()?;

// Store a secret
manager.store_secret("api-key", "sk-1234567890")?;

// Retrieve a secret
let secret = manager.get_secret("api-key")?;

// Rotate a secret
manager.rotate_secret("api-key", "sk-0987654321")?;

// Check if rotation is needed
if manager.needs_rotation("api-key")? {
    manager.rotate_secret("api-key", "new-secret")?;
}

// List all secrets
let secrets = manager.list_secrets();

// Get metadata
let metadata = manager.get_metadata("api-key")?;

// Delete a secret
manager.delete_secret("api-key")?;
```

#### FFI API

```c
// Initialize secret manager
mcp_security_init_secret_manager();

// Store a secret
char* result = mcp_security_store_secret("api-key", "sk-1234567890");
// Process result...
mcp_free_string(result);

// Retrieve a secret
char* secret = mcp_security_get_secret("api-key");
// Process secret...
mcp_free_string(secret);

// Rotate a secret
char* result = mcp_security_rotate_secret("api-key", "sk-0987654321");
// Process result...
mcp_free_string(result);

// List secrets
char* secrets = mcp_security_list_secrets();
// Process secrets...
mcp_free_string(secrets);

// Get metadata
char* metadata = mcp_security_get_secret_metadata("api-key");
// Process metadata...
mcp_free_string(metadata);

// Delete a secret
char* result = mcp_security_delete_secret("api-key");
// Process result...
mcp_free_string(result);

// Get configuration
char* config = mcp_security_get_secrets_config();
// Process config...
mcp_free_string(config);

// Set configuration
char* new_config = "{\"auto_rotate\": true, ...}";
char* result = mcp_security_set_secrets_config(new_config);
// Process result...
mcp_free_string(result);
```

## Security Best Practices

### Input Validation

1. **Always validate user input** before processing
2. **Use URL validation** for all external URLs
3. **Sanitize HTML** when displaying user-generated content
4. **Validate JSON structure** for all JSON inputs
5. **Configure appropriate limits** based on your use case

### Rate Limiting

1. **Set appropriate limits** based on your expected traffic
2. **Monitor usage** to detect abuse patterns
3. **Use burst allowance** for legitimate traffic spikes
4. **Clean up inactive sessions** regularly
5. **Implement backoff** for rate-limited clients

### Secrets Management

1. **Never hardcode secrets** in your application
2. **Use encryption at rest** for all sensitive data
3. **Rotate secrets regularly** based on your security policy
4. **Use versioning** to track secret changes
5. **Implement proper key derivation** for encryption keys

## Configuration via FFI

All security parameters can be configured dynamically via FFI:

### Example: Update Validation Configuration

```json
{
  "max_input_size_bytes": 5242880,
  "max_message_length": 50000,
  "max_concurrent_tool_calls": 5,
  "allowed_url_patterns": ["^https?://[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}.*$"],
  "blocked_url_patterns": ["^file://.*$", "^data:.*$"],
  "sanitize_html": true,
  "validate_json_schema": true,
  "max_json_nesting_depth": 10,
  "max_json_array_length": 1000,
  "blocked_keywords": ["<script", "javascript:", "eval("]
}
```

### Example: Update Rate Limit Configuration

```json
{
  "enabled": true,
  "requests_per_minute": 100,
  "requests_per_hour": 1000,
  "requests_per_day": 10000,
  "max_concurrent_sessions": 10,
  "window_size_secs": 60,
  "burst_allowance": 10,
  "cleanup_interval_secs": 300
}
```

### Example: Update Secrets Configuration

```json
{
  "enabled": true,
  "encrypt_at_rest": true,
  "encryption_algorithm": "AES256-GCM",
  "key_derivation_function": "Argon2",
  "key_derivation_iterations": 100000,
  "auto_rotate": true,
  "rotation_interval_hours": 720,
  "max_secret_age_hours": 2160,
  "storage_backend": "memory",
  "storage_path": null,
  "enable_versioning": true,
  "max_versions": 5
}
```

## Security Logging

The `SecurityLogger` struct provides structured logging for security events:

### Methods

- **`rate_limit_check(session_id: &str)`** — Logs rate limit check attempts
- **`rate_limit_exceeded(session_id: &str)`** — Logs rate limit exceeded events
- **`secret_stored(key: &str)`** — Logs secret storage operations
- **`secret_retrieved(key: &str)`** — Logs secret retrieval operations
- **`secret_rotated(key: &str)`** — Logs secret rotation events
- **`secret_deleted(key: &str)`** — Logs secret deletion events
- **`secret_error(key: &str, error: &str)`** — Logs secret-related errors
- **`cleanup_task()`** — Logs cleanup task execution

## Testing

Comprehensive unit tests are provided for all security modules:

```bash
# Run security tests
cargo test -p antikythera-core security

# Run FFI tests
cargo test -p antikythera-sdk security_ffi
```

### Test Files

- `tests/security/mod.rs`
- `tests/security/config_tests.rs`
- `tests/security/validation_tests.rs`
- `tests/security/rate_limit_tests.rs`
- `tests/security/secrets_tests.rs`

## CLI vs Core Separation

The security implementation follows the principle of separation of concerns:

- **Core Module**: Contains all security logic and is completely agnostic of CLI concerns
- **CLI Module**: Only handles user interaction and configuration management
- **FFI Interface**: Provides a clean boundary for host languages to interact with security features

This ensures that the core security functionality can be used independently of the CLI and can be embedded in any host application.

## Future Enhancements

Potential future security features:

1. **Advanced Threat Detection**: ML-based anomaly detection
2. **IP-based Rate Limiting**: Per-IP rate limiting
3. **OAuth Integration**: Support for OAuth token management
4. **Hardware Security Modules**: Integration with HSMs for key storage
5. **Audit Logging**: Comprehensive security event logging
6. **Compliance Reporting**: Built-in compliance reporting tools

## Security Considerations

### Important Notes

1. **Encryption**: The current implementation uses simplified encryption. For production use, integrate with proper cryptographic libraries like `rustls` or `ring`.

2. **Secret Storage**: The memory backend stores secrets in RAM. For production use, consider using a secure vault solution like HashiCorp Vault or AWS Secrets Manager.

3. **Rate Limiting**: Rate limits are enforced in-memory. For distributed systems, consider using Redis or another distributed cache.

4. **Input Validation**: While comprehensive, input validation cannot prevent all attacks. Always use defense in depth.

5. **Thread Safety**: The current implementation uses global static variables for FFI compatibility. For multi-threaded hosts, ensure proper synchronization.

## License

This security module is part of the Antikythera MCP Framework and is licensed under the same terms as the main project.


