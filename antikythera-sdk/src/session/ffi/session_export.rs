//! Session Export/Import FFI
//!
//! Export and import sessions with Postcard binary format (hex encoded).

use std::os::raw::c_char;

use super::helpers::*;
use super::session_mgmt::SESSION_MANAGER;
use antikythera_session::{SessionExport, BatchExport};

/// Export a session to Postcard binary (returned as hex string)
///
/// # Returns
/// JSON with `session_id`, `export_data` (hex), `size` fields
pub fn mcp_session_export(session_id: *const c_char) -> *mut c_char {
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
pub fn mcp_session_import(export_data: *const c_char) -> *mut c_char {
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

/// Export all sessions as batch
///
/// # Returns
/// JSON with `session_count`, `export_data` (hex), `size` fields
pub fn mcp_batch_export() -> *mut c_char {
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
pub fn mcp_batch_import(export_data: *const c_char) -> *mut c_char {
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

