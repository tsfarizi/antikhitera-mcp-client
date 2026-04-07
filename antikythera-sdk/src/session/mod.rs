//! Session Management Module
//!
//! Wraps antikythera-session for SDK usage with FFI bindings.
//! Integrates with antikythera-log for session-specific logging.

mod ffi;

// Re-export FFI functions
pub use ffi::{
    // Session management
    mcp_session_create, mcp_session_get, mcp_session_list,
    mcp_session_add_message, mcp_session_get_history,
    mcp_session_export, mcp_session_import,
    mcp_session_delete, mcp_session_clear,
    mcp_batch_export, mcp_batch_import,
    // Session log integration
    mcp_session_export_logs, mcp_session_import_logs,
    mcp_session_get_logs, mcp_session_batch_export_logs,
    mcp_session_batch_import_logs,
};

// Re-export session types from antikythera-session
pub use antikythera_session::{
    Session, SessionSummary,
    SessionExport, BatchExport,
};

/// Message types for session
pub use antikythera_session::Message;
pub use antikythera_session::MessageRole;

/// Session log export types from antikythera-log
pub use antikythera_log::{SessionLogExport, BatchLogExport};

/// Thread-safe session manager for SDK usage
pub struct SdkSessionManager {
    inner: antikythera_session::SessionManager,
}

impl SdkSessionManager {
    pub fn new() -> Self {
        Self {
            inner: antikythera_session::SessionManager::new(),
        }
    }

    pub fn create_session(&self, user_id: &str, model: &str) -> String {
        self.inner.create_session(user_id, model)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.inner.get_session(session_id)
    }

    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        self.inner.list_sessions()
    }

    pub fn add_message(&self, session_id: &str, message: Message) -> Result<(), String> {
        self.inner.add_message(session_id, message)
    }

    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<Message>, String> {
        self.inner.get_chat_history(session_id)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        self.inner.delete_session(session_id)
    }

    pub fn clear_session(&self, session_id: &str) -> Result<(), String> {
        self.inner.clear_session(session_id)
    }
}

impl Default for SdkSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
