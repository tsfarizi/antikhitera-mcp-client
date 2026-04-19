//! Configuration FFI Module
//!
//! Full configuration management via FFI following Vertical Slice Architecture.
//! All config is stored as Postcard binary format.
//!
//! ## Module Structure
//!
//! ```text
//! config_ffi/
//! ├── mod.rs       # This file - module exports
//! ├── helpers.rs   # Common FFI utilities
//! ├── core.rs      # Core config operations (init, get/set all, export, import, reset)
//! ├── fields.rs    # Field-level operations (get/set by path)
//! ├── providers.rs # Provider management (add, remove, list)
//! ├── prompts.rs   # Prompt template management (get, set, list)
//! └── agent.rs     # Agent configuration (get, set_*)
//! ```
//!
//! ## Usage
//!
//! Functions are exposed as Rust APIs and can be wrapped by host-specific bindings.

// Internal modules
mod helpers;
mod core;
mod fields;
mod providers;
mod prompts;
mod agent;

// Re-export config module for internal use
pub(super) use super::config;

// ============================================================================
// Public API - Re-export all FFI functions
// ============================================================================

// Core config operations
pub use core::{
    mcp_config_init,
    mcp_config_exists,
    mcp_config_size,
    mcp_config_get_all,
    mcp_config_set_all,
    mcp_config_export,
    mcp_config_import,
    mcp_config_reset,
    mcp_config_use_from,
    mcp_config_backup_to,
};

// Field-level operations
pub use fields::{
    mcp_config_get,
    mcp_config_set,
};

// Provider management
pub use providers::{
    mcp_config_add_provider,
    mcp_config_remove_provider,
    mcp_config_list_providers,
    mcp_config_set_provider_api_key,
    mcp_config_get_provider_api_key,
    mcp_config_add_provider_model,
    mcp_config_remove_provider_model,
    mcp_config_list_provider_models,
};

// Prompt management
pub use prompts::{
    mcp_config_get_prompt,
    mcp_config_set_prompt,
    mcp_config_list_prompts,
};

// Agent configuration
pub use agent::{
    mcp_config_get_agent,
    mcp_config_set_agent_max_steps,
    mcp_config_set_agent_verbose,
    mcp_config_set_agent_auto_execute,
};
