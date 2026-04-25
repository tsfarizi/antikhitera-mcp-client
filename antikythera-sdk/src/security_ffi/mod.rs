//! Security FFI Module
//!
//! Exposes security features (validation, rate limiting, secrets management) to host languages via C FFI.
//!
//! ## Module Structure
//!
//! ```text
//! security_ffi/
//! ├── mod.rs              # This file - module exports
//! ├── helpers.rs          # Common FFI utilities
//! ├── validation.rs       # Input validation FFI
//! ├── rate_limit.rs       # Rate limiting FFI
//! └── secrets.rs          # Secrets management FFI
//! ```

mod helpers;
mod rate_limit;
mod secrets;
mod validation;

// Re-export FFI functions
pub use helpers::mcp_security_free_string;

pub use validation::{
    mcp_security_get_validation_config, mcp_security_init_validator, mcp_security_sanitize_html,
    mcp_security_set_validation_config, mcp_security_validate_input, mcp_security_validate_json,
    mcp_security_validate_url,
};

pub use rate_limit::{
    mcp_security_check_rate_limit, mcp_security_get_rate_limit_config, mcp_security_get_usage,
    mcp_security_init_rate_limiter, mcp_security_remove_session, mcp_security_reset_session,
    mcp_security_set_rate_limit_config,
};

pub use secrets::{
    mcp_security_delete_secret, mcp_security_get_secret, mcp_security_get_secret_metadata,
    mcp_security_get_secrets_config, mcp_security_init_secret_manager, mcp_security_list_secrets,
    mcp_security_rotate_secret, mcp_security_set_secrets_config, mcp_security_store_secret,
};
