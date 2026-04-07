//! Session Manager
//!
//! Manages multiple concurrent sessions with thread-safe operations.

use crate::session::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
/// Session Manager
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
    /// Session Creation
    // ========================================================================

    /// Create a new session
    pub fn create_session(&self, user_id: impl Into<String>, model: impl Into<String>) -> String {
        let session = Session::new(user_id, model);
        let id = session.id.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(id.clone(), session);

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
        session.id = session_id.into();
        let id = session.id.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(id.clone(), session);

        id
    }

    // ========================================================================
    /// Session Access
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
    /// Message Operations
    // ========================================================================

    /// Add a message to a session
    pub fn add_message(&self, session_id: &str, message: Message) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.add_message(message);
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

    /// Get latest message
    pub fn get_latest_message(&self, session_id: &str) -> Result<Option<Message>, String> {
        let sessions = self.sessions.read().unwrap();
        match sessions.get(session_id) {
            Some(session) => Ok(session.latest_message().cloned()),
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    // ========================================================================
    /// Session Management
    // ========================================================================

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.remove(session_id) {
            Some(_) => Ok(()),
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Clear all messages in a session
    pub fn clear_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.clear_messages();
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    /// Update session metadata
    pub fn update_metadata(&self, session_id: &str, metadata: impl Into<String>) -> Result<(), String> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.metadata = Some(metadata.into());
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
                Ok(())
            }
            None => Err(format!("Session not found: {}", session_id)),
        }
    }

    // ========================================================================
    /// Query Operations
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

    /// Get sessions by model
    pub fn get_sessions_by_model(&self, model: &str) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| s.model == model)
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

    /// Clear all sessions
    pub fn clear_all(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.clear();
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
