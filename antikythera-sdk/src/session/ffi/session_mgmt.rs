//! Session Management FFI
//!
//! Create, get, list, delete, and clear sessions.

use std::os::raw::c_char;
use std::sync::LazyLock;

use super::helpers::*;
use crate::session::SdkSessionManager;

/// Global session manager instance
pub static SESSION_MANAGER: LazyLock<SdkSessionManager> = LazyLock::new(SdkSessionManager::new);

/// Create a new session
///
/// # Returns
/// JSON with `session_id`, `user_id`, `model` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_create(user_id: *const c_char, model: *const c_char) -> *mut c_char {
    let user_str = match from_c_string(user_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let model_str = match from_c_string(model) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let session_id = SESSION_MANAGER.create_session(&user_str, &model_str);

    success_with(&[
        ("session_id", serde_json::json!(session_id)),
        ("user_id", serde_json::json!(user_str)),
        ("model", serde_json::json!(model_str)),
    ])
}

/// Get session by ID
///
/// # Returns
/// JSON Session object or error
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_get(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.get_session(&id_str) {
        Some(session) => serialize_result(&session),
        None => error_response(&format!("Session not found: {}", id_str)),
    }
}

/// List all sessions
///
/// # Returns
/// JSON array of SessionSummary objects
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_list() -> *mut c_char {
    let sessions = SESSION_MANAGER.list_sessions();
    serialize_result(&sessions)
}

/// Delete a session
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_delete(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.delete_session(&id_str) {
        Ok(()) => success_with(&[("session_id", serde_json::json!(id_str))]),
        Err(e) => error_response(&e),
    }
}

/// Clear session messages
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_clear(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.clear_session(&id_str) {
        Ok(()) => success_with(&[("session_id", serde_json::json!(id_str))]),
        Err(e) => error_response(&e),
    }
}
