//! Session FFI Bindings
//!
//! Exposes session management to host languages via C FFI.
//! Split into modules for better separation of concerns:
//! - helpers: Common utilities (internal)
//! - session_mgmt: Create, get, list, delete, clear
//! - session_messages: Add messages, get history
//! - session_export: Export/import sessions
//! - session_logs: Export/import session logs

mod helpers;
mod session_export;
mod session_logs;
mod session_messages;
mod session_mgmt;

// Re-export FFI functions only (not internal helpers)
pub use session_export::{
    mcp_batch_export, mcp_batch_import, mcp_session_export, mcp_session_import,
};
pub use session_logs::{
    mcp_session_batch_export_logs, mcp_session_batch_import_logs, mcp_session_export_logs,
    mcp_session_get_logs, mcp_session_import_logs,
};
pub use session_messages::{mcp_session_add_message, mcp_session_get_history};
pub use session_mgmt::{
    mcp_session_clear, mcp_session_create, mcp_session_delete, mcp_session_get, mcp_session_list,
};
