//! Session Message FFI
//!
//! Add messages and get chat history.

use std::os::raw::c_char;

use super::helpers::*;
use super::session_mgmt::SESSION_MANAGER;
use antikythera_session::Message;

/// Add a message to a session
///
/// # Returns
/// JSON with `success` and `message_count` fields
pub fn mcp_session_add_message(
    session_id: *const c_char,
    role: *const c_char,
    content: *const c_char,
) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let role_str = match from_c_string(role) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let content_str = match from_c_string(content) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Create message based on role
    let message = match role_str.as_str() {
        "user" => Message::user(&content_str),
        "assistant" => Message::assistant(&content_str),
        "system" => Message::system(&content_str),
        _ => return error_response(&format!("Unknown role: {}", role_str)),
    };

    match SESSION_MANAGER.add_message(&id_str, message) {
        Ok(()) => {
            let history = SESSION_MANAGER.get_chat_history(&id_str).unwrap_or_default();
            success_with(&[
                ("session_id", serde_json::json!(id_str)),
                ("message_count", serde_json::json!(history.len())),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Get chat history for a session
///
/// # Returns
/// JSON array of Message objects
pub fn mcp_session_get_history(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.get_chat_history(&id_str) {
        Ok(history) => serialize_result(&history),
        Err(e) => error_response(&e),
    }
}

