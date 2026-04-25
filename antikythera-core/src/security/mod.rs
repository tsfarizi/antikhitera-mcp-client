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
//! ├── validation.rs       # Input validation and sanitization
//! ├── rate_limit.rs       # Rate limiting with configurable parameters
//! ├── secrets.rs          # Secure secrets storage and rotation
//! └── config.rs           # Security configuration types
//! ```

pub mod config;
pub mod rate_limit;
pub mod secrets;
pub mod validation;

pub use config::{RateLimitConfig, SecretsConfig, SecurityConfig, ValidationConfig};
pub use rate_limit::RateLimiter;
pub use secrets::{SecretManager, SecretRotationPolicy};
pub use validation::{InputValidator, ValidationError, ValidationResult};
