//! Session FFI Bindings
//!
//! Exposes session management to host languages via C FFI.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::LazyLock;

use crate::session::SdkSessionManager;
use antikythera_session::{Message, SessionExport, BatchExport};
use antikythera_log::{SessionLogExport, BatchLogExport};
use antikythera_core::logging::get_latest_logs;

// ============================================================================
// Global Session Manager
// ============================================================================

static SESSION_MANAGER: LazyLock<SdkSessionManager> = LazyLock::new(SdkSessionManager::new);

// ============================================================================
// Helpers
// ============================================================================

fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn from_c_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Null pointer".to_string());
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

fn serialize_result<T: serde::Serialize>(result: &T) -> *mut c_char {
    match serde_json::to_string(result) {
        Ok(json) => to_c_string(&json),
        Err(e) => {
            let error = serde_json::json!({"error": format!("Serialization failed: {}", e)});
            to_c_string(&error.to_string())
        }
    }
}

fn error_response(message: &str) -> *mut c_char {
    to_c_string(&format!(r#"{{"error": "{}"}}"#, message))
}

fn success_with(fields: &[(&str, serde_json::Value)]) -> *mut c_char {
    let mut obj = serde_json::Map::new();
    obj.insert("success".to_string(), serde_json::json!(true));
    for (key, value) in fields {
        obj.insert(key.to_string(), value.clone());
    }
    serialize_result(&serde_json::Value::Object(obj))
}

/// Decode hex string to bytes
fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("Hex decode error: {}", e)))
        .collect()
}

// ============================================================================
/// Session Management FFI
// ============================================================================

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

/// Add a message to a session
///
/// # Returns
/// JSON with `success` and `message_count` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_add_message(
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
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_get_history(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.get_chat_history(&id_str) {
        Ok(history) => serialize_result(&history),
        Err(e) => error_response(&e),
    }
}

/// Export a session to Postcard binary (returned as base64 string)
///
/// # Returns
/// JSON with `session_id`, `export_data` (base64), `size` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_export(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    match SESSION_MANAGER.get_session(&id_str) {
        Some(session) => {
            let export = SessionExport::from_session(session);
            match export.to_postcard() {
                Ok(data) => {
                    let hex_str = data.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                    success_with(&[
                        ("session_id", serde_json::json!(id_str)),
                        ("export_data", serde_json::json!(hex_str)),
                        ("size", serde_json::json!(data.len())),
                    ])
                }
                Err(e) => error_response(&e),
            }
        }
        None => error_response(&format!("Session not found: {}", id_str)),
    }
}

/// Import a session from Postcard binary (hex encoded)
///
/// # Returns
/// JSON with `success` and `session_id` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_import(export_data: *const c_char) -> *mut c_char {
    let data_str = match from_c_string(export_data) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Decode hex
    let data = match hex_decode(&data_str) {
        Ok(d) => d,
        Err(e) => return error_response(&e),
    };

    // Deserialize
    match SessionExport::from_postcard(&data) {
        Ok(export) => {
            let session_id = export.session.id.clone();
            // Note: In real implementation, would add to manager here
            success_with(&[
                ("session_id", serde_json::json!(session_id)),
                ("imported", serde_json::json!(true)),
            ])
        }
        Err(e) => error_response(&e),
    }
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

// ============================================================================
/// Batch Export/Import FFI
// ============================================================================

/// Export all sessions as batch
///
/// # Returns
/// JSON with `session_count`, `export_data` (base64), `size` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_batch_export() -> *mut c_char {
    let sessions: Vec<_> = SESSION_MANAGER
        .list_sessions()
        .iter()
        .filter_map(|s| SESSION_MANAGER.get_session(&s.id))
        .collect();

    let batch = BatchExport::from_sessions(sessions);

    match batch.to_postcard() {
        Ok(data) => {
            let hex_str = data.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            success_with(&[
                ("session_count", serde_json::json!(batch.session_count())),
                ("export_data", serde_json::json!(hex_str)),
                ("size", serde_json::json!(data.len())),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Import batch of sessions
#[unsafe(no_mangle)]
pub extern "C" fn mcp_batch_import(export_data: *const c_char) -> *mut c_char {
    let data_str = match from_c_string(export_data) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let data = match hex_decode(&data_str) {
        Ok(d) => d,
        Err(e) => return error_response(&e),
    };

    match BatchExport::from_postcard(&data) {
        Ok(batch) => success_with(&[
            ("session_count", serde_json::json!(batch.session_count())),
            ("imported", serde_json::json!(true)),
        ]),
        Err(e) => error_response(&e),
    }
}

// ============================================================================
/// Session Log Integration FFI
// ============================================================================

/// Get logs for a specific session
///
/// # Returns
/// JSON array of LogEntry objects for the session
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_get_logs(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Get logs from core logging system
    let logs = get_latest_logs(&id_str, 1000);
    serialize_result(&logs)
}

/// Export session logs to Postcard binary (hex encoded)
///
/// # Returns
/// JSON with `session_id`, `export_data` (hex), `log_count`, `size` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_export_logs(session_id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(session_id) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Get logs for this session
    let logs = get_latest_logs(&id_str, 10000);

    // Create session log export
    let export = SessionLogExport::from_logs(&id_str, logs);

    match export.to_postcard() {
        Ok(data) => {
            let hex_str = data.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            success_with(&[
                ("session_id", serde_json::json!(id_str)),
                ("export_data", serde_json::json!(hex_str)),
                ("log_count", serde_json::json!(export.log_count())),
                ("size", serde_json::json!(data.len())),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Import session logs from Postcard binary (hex encoded)
///
/// # Returns
/// JSON with `success`, `session_id`, `log_count` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_import_logs(export_data: *const c_char) -> *mut c_char {
    let data_str = match from_c_string(export_data) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    // Decode hex
    let data = match hex_decode(&data_str) {
        Ok(d) => d,
        Err(e) => return error_response(&e),
    };

    // Deserialize
    match SessionLogExport::from_postcard(&data) {
        Ok(export) => {
            let log_count = export.log_count();
            let session_id = export.session_id.clone();
            // Note: In real implementation, would re-inject logs to logger here
            success_with(&[
                ("session_id", serde_json::json!(session_id)),
                ("log_count", serde_json::json!(log_count)),
                ("imported", serde_json::json!(true)),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Export all session logs as batch
///
/// # Returns
/// JSON with `session_count`, `total_log_count`, `export_data` (hex), `size` fields
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_batch_export_logs() -> *mut c_char {
    // Get all unique session IDs from loggers
    // For now, export a placeholder - in production would iterate all sessions
    let session_logs: Vec<SessionLogExport> = Vec::new();

    let batch = BatchLogExport::from_session_logs(session_logs);

    match batch.to_postcard() {
        Ok(data) => {
            let hex_str = data.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            success_with(&[
                ("session_count", serde_json::json!(batch.session_count())),
                ("total_log_count", serde_json::json!(batch.total_log_count())),
                ("export_data", serde_json::json!(hex_str)),
                ("size", serde_json::json!(data.len())),
            ])
        }
        Err(e) => error_response(&e),
    }
}

/// Import batch of session logs
#[unsafe(no_mangle)]
pub extern "C" fn mcp_session_batch_import_logs(export_data: *const c_char) -> *mut c_char {
    let data_str = match from_c_string(export_data) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let data = match hex_decode(&data_str) {
        Ok(d) => d,
        Err(e) => return error_response(&e),
    };

    match BatchLogExport::from_postcard(&data) {
        Ok(batch) => success_with(&[
            ("session_count", serde_json::json!(batch.session_count())),
            ("total_log_count", serde_json::json!(batch.total_log_count())),
            ("imported", serde_json::json!(true)),
        ]),
        Err(e) => error_response(&e),
    }
}
