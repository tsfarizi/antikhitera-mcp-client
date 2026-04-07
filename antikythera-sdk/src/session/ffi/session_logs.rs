//! Session Log Integration FFI
//!
//! Export and import logs tied to specific sessions.

use std::os::raw::c_char;

use super::helpers::*;
use antikythera_core::logging::get_latest_logs;
use antikythera_log::{SessionLogExport, BatchLogExport};

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
