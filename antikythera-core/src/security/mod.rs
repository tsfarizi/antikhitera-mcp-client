//! Security Module
//!
//! Comprehensive security features for input validation, rate limiting,
//! and secrets management. All security parameters are configurable via FFI.
//!
//! ## Module Structure
//!
//! ```text
//! security/
//! ├── mod.rs              # This file - module exports
//! ├── config.rs           # Security configuration types
//! ├── validation/
//! │   ├── mod.rs          # Input validation and sanitization
//! │   ├── json.rs         # JSON structure validation
//! │   ├── types.rs        # Validation result types
//! │   └── url.rs          # URL validation and sanitization
//! ├── rate_limit.rs       # Rate limiting with configurable parameters
//! └── secrets/
//!     ├── mod.rs          # Secure secrets storage and rotation
//!     ├── crypto.rs       # Encryption/decryption (AES-256-GCM)
//!     ├── error.rs        # Secret management errors
//!     └── storage.rs      # Storage backends (memory, file)
//! ```

pub mod config;
pub mod rate_limit;
pub mod secrets;
pub mod validation;

pub use config::{RateLimitConfig, SecretsConfig, SecurityConfig, ValidationConfig};
pub use rate_limit::RateLimiter;
pub use secrets::{SecretManager, SecretRotationPolicy};
pub use validation::{InputValidator, InputValidatorError, ValidationError, ValidationResult};
