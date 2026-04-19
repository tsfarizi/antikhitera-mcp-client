//! Agent Configuration FFI
//!
//! Get and set agent behavior settings.

use std::os::raw::c_char;
use super::config;
use super::helpers::*;

/// Get current agent configuration
///
/// # Returns
/// JSON AgentConfig object with all agent settings
pub fn mcp_config_get_agent() -> *mut c_char {
    let cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    serialize_result(&cfg.agent)
}

/// Set agent maximum interaction steps
///
/// # Parameters
/// - `steps`: Maximum number of tool interaction steps allowed
///
/// # Returns
/// JSON with `success` and `max_steps` fields
pub fn mcp_config_set_agent_max_steps(steps: u32) -> *mut c_char {
    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    cfg.agent.max_steps = steps;

    match config::save_config(&cfg, None) {
        Ok(()) => serialize_result(&serde_json::json!({
            "success": true,
            "max_steps": steps
        })),
        Err(e) => error_response(&e),
    }
}

/// Toggle agent verbose logging
///
/// # Parameters
/// - `enabled`: 1 to enable verbose logging, 0 to disable
///
/// # Returns
/// JSON with `success` and `verbose` fields
pub fn mcp_config_set_agent_verbose(enabled: i32) -> *mut c_char {
    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    cfg.agent.verbose = enabled != 0;

    match config::save_config(&cfg, None) {
        Ok(()) => serialize_result(&serde_json::json!({
            "success": true,
            "verbose": cfg.agent.verbose
        })),
        Err(e) => error_response(&e),
    }
}

/// Toggle automatic tool execution
///
/// # Parameters
/// - `enabled`: 1 to enable auto-execute, 0 to disable
///
/// # Returns
/// JSON with `success` and `auto_execute_tools` fields
pub fn mcp_config_set_agent_auto_execute(enabled: i32) -> *mut c_char {
    let mut cfg = match config::load_config(None) {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };

    cfg.agent.auto_execute_tools = enabled != 0;

    match config::save_config(&cfg, None) {
        Ok(()) => serialize_result(&serde_json::json!({
            "success": true,
            "auto_execute_tools": cfg.agent.auto_execute_tools
        })),
        Err(e) => error_response(&e),
    }
}

