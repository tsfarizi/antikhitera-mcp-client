//! Session Manager
//!
//! Manages multiple concurrent sessions with thread-safe operations.

use crate::session::*;
use antikythera_log::{Logger, LogLevel};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Session Manager
// ============================================================================

/// Thread-safe session manager supporting concurrent operations
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========================================================================
    // Session Creation
    // ========================================================================

    /// Create a new session
    pub fn create_session(&self, user_id: impl Into<String>, model: impl Into<String>) -> String {
        let session = Session::new(user_id, model);
        let id = session.id.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(id.clone(), session);

        let log = Logger::new(&id);
        log.log_with_source(LogLevel::Info, "session", format!("Session created | id={}", id));

        id
    }

    /// Create session with custom ID
    pub fn create_session_with_id(
        &self,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        model: impl Into<String>,
    ) -> String {
        let mut session = Session::new(user_id, model);
        let id = session_id.into();
        session.id = id.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(id.clone(), session);

        let log = Logger::new(&id);
        log.log_with_source(LogLevel::Info, "session", format!("Session created with custom ID | id={}", id));

        id
    }

    // ========================================================================
    // Session Access
    // ========================================================================

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().unwrap();
        sessions.contains_key(session_id)
    }

    /// List all session summaries
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        sessions.values().map(SessionSummary::from).collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.len()
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Add a message to a session
    pub fn add_message(&self, session_id: &str, message: Message) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.add_message(message);
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Debug, "session", format!("Message appended | id={}", session_id));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Get chat history for a session
    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<Message>, String> {
        let sessions = self.sessions.read().unwrap();
        match sessions.get(session_id) {
            Some(session) => Ok(session.messages.clone()),
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.remove(session_id) {
            Some(_) => {
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Info, "session", format!("Session deleted | id={}", session_id));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Clear all messages in a session
    pub fn clear_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.clear_messages();
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Debug, "session", format!("History cleared | id={}", session_id));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Update session title
    pub fn update_title(&self, session_id: &str, title: impl Into<String>) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.set_title(title);
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Debug, "session", format!("Title updated | id={}", session_id));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Record token usage
    pub fn record_tokens(&self, session_id: &str, tokens: u64) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.add_tokens(tokens);
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Debug, "session", format!("Tokens recorded | id={} | tokens={}", session_id, tokens));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Record tool usage
    pub fn record_tool(&self, session_id: &str, tool_name: &str, step: u32) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.record_tool(tool_name, step);
                let log = Logger::new(session_id);
                log.log_with_source(LogLevel::Debug, "session", format!("Tool recorded | id={} | tool={} | step={}", session_id, tool_name, step));
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get sessions by user ID
    pub fn get_sessions_by_user(&self, user_id: &str) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .map(SessionSummary::from)
            .collect()
    }

    /// Search sessions by title
    pub fn search_sessions(&self, query: &str) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| {
                s.title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&query.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(SessionSummary::from)
            .collect()
    }

    /// Import a session into the manager, replacing any existing one.
    pub fn import_session(&self, session: Session) -> Result<(), String> {
        let session_id = session.id.clone();
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), session);
        let log = Logger::new(&session_id);
        log.log_with_source(LogLevel::Info, "session", format!("Session imported | id={}", session_id));
        Ok(())
    }

    /// Import many sessions into the manager.
    pub fn import_sessions(&self, imported_sessions: Vec<Session>) -> Result<usize, String> {
        let count = imported_sessions.len();
        let mut sessions = self.sessions.write().unwrap();
        for session in imported_sessions {
            let id = session.id.clone();
            sessions.insert(id.clone(), session);
            let log = Logger::new(&id);
            log.log_with_source(LogLevel::Info, "session", format!("Session imported | id={}", id));
        }
        Ok(count)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
