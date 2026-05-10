//! Session Manager
//!
//! Manages multiple concurrent sessions with thread-safe operations.

use crate::session::*;
use antikythera_log::{LogLevel, Logger};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Session Manager Error
// ============================================================================

/// Errors that can occur during session manager operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionManagerError {
    LockPoisoned(String),
    SessionNotFound(String),
    Other(String),
}

impl std::fmt::Display for SessionManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionManagerError::LockPoisoned(msg) => write!(f, "Lock poisoned: {}", msg),
            SessionManagerError::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
            SessionManagerError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SessionManagerError {}

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
    pub fn create_session(
        &self,
        user_id: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<String, SessionManagerError> {
        let session = Session::new(user_id, model);
        let id = session.id.clone();

        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("create_session: {}", e)))?;
        sessions.insert(id.clone(), session);

        let log = Logger::new(&id);
        log.log_with_source(
            LogLevel::Info,
            "session",
            format!("Session created | id={}", id),
        );

        Ok(id)
    }

    /// Create session with custom ID
    pub fn create_session_with_id(
        &self,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<String, SessionManagerError> {
        let mut session = Session::new(user_id, model);
        let id = session_id.into();
        session.id = id.clone();

        let mut sessions = self.sessions.write().map_err(|e| {
            SessionManagerError::LockPoisoned(format!("create_session_with_id: {}", e))
        })?;
        sessions.insert(id.clone(), session);

        let log = Logger::new(&id);
        log.log_with_source(
            LogLevel::Info,
            "session",
            format!("Session created with custom ID | id={}", id),
        );

        Ok(id)
    }

    // ========================================================================
    // Session Access
    // ========================================================================

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("get_session: {}", e)))?;
        Ok(sessions.get(session_id).cloned())
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> Result<bool, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("has_session: {}", e)))?;
        Ok(sessions.contains_key(session_id))
    }

    /// List all session summaries
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("list_sessions: {}", e)))?;
        Ok(sessions.values().map(SessionSummary::from).collect())
    }

    /// Get session count
    pub fn session_count(&self) -> Result<usize, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("session_count: {}", e)))?;
        Ok(sessions.len())
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Add a message to a session
    pub fn add_message(
        &self,
        session_id: &str,
        message: Message,
    ) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("add_message: {}", e)))?;
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.add_message(message);
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Debug,
                    "session",
                    format!("Message appended | id={}", session_id),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "add_message: {}",
                session_id
            ))),
        }
    }

    /// Get chat history for a session
    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<Message>, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("get_chat_history: {}", e)))?;
        match sessions.get(session_id) {
            Some(session) => Ok(session.messages.clone()),
            None => Err(SessionManagerError::SessionNotFound(format!(
                "get_chat_history: {}",
                session_id
            ))),
        }
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("delete_session: {}", e)))?;
        match sessions.remove(session_id) {
            Some(_) => {
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Info,
                    "session",
                    format!("Session deleted | id={}", session_id),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "delete_session: {}",
                session_id
            ))),
        }
    }

    /// Clear all messages in a session
    pub fn clear_session(&self, session_id: &str) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("clear_session: {}", e)))?;
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.clear_messages();
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Debug,
                    "session",
                    format!("History cleared | id={}", session_id),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "clear_session: {}",
                session_id
            ))),
        }
    }

    /// Update session title
    pub fn update_title(
        &self,
        session_id: &str,
        title: impl Into<String>,
    ) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("update_title: {}", e)))?;
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.set_title(title);
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Debug,
                    "session",
                    format!("Title updated | id={}", session_id),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "update_title: {}",
                session_id
            ))),
        }
    }

    /// Record token usage
    pub fn record_tokens(&self, session_id: &str, tokens: u64) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("record_tokens: {}", e)))?;
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.add_tokens(tokens);
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Debug,
                    "session",
                    format!("Tokens recorded | id={} | tokens={}", session_id, tokens),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "record_tokens: {}",
                session_id
            ))),
        }
    }

    /// Record tool usage
    pub fn record_tool(
        &self,
        session_id: &str,
        tool_name: &str,
        step: u32,
    ) -> Result<(), SessionManagerError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("record_tool: {}", e)))?;
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.record_tool(tool_name, step);
                let log = Logger::new(session_id);
                log.log_with_source(
                    LogLevel::Debug,
                    "session",
                    format!(
                        "Tool recorded | id={} | tool={} | step={}",
                        session_id, tool_name, step
                    ),
                );
                Ok(())
            }
            None => Err(SessionManagerError::SessionNotFound(format!(
                "record_tool: {}",
                session_id
            ))),
        }
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get sessions by user ID
    pub fn get_sessions_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<SessionSummary>, SessionManagerError> {
        let sessions = self.sessions.read().map_err(|e| {
            SessionManagerError::LockPoisoned(format!("get_sessions_by_user: {}", e))
        })?;
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .map(SessionSummary::from)
            .collect())
    }

    /// Search sessions by title
    pub fn search_sessions(&self, query: &str) -> Result<Vec<SessionSummary>, SessionManagerError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("search_sessions: {}", e)))?;
        Ok(sessions
            .values()
            .filter(|s| {
                s.title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&query.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(SessionSummary::from)
            .collect())
    }

    /// Import a session into the manager, replacing any existing one.
    pub fn import_session(&self, session: Session) -> Result<(), SessionManagerError> {
        let session_id = session.id.clone();
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("import_session: {}", e)))?;
        sessions.insert(session_id.clone(), session);
        let log = Logger::new(&session_id);
        log.log_with_source(
            LogLevel::Info,
            "session",
            format!("Session imported | id={}", session_id),
        );
        Ok(())
    }

    /// Import many sessions into the manager.
    pub fn import_sessions(
        &self,
        imported_sessions: Vec<Session>,
    ) -> Result<usize, SessionManagerError> {
        let count = imported_sessions.len();
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| SessionManagerError::LockPoisoned(format!("import_sessions: {}", e)))?;
        for session in imported_sessions {
            let id = session.id.clone();
            sessions.insert(id.clone(), session);
            let log = Logger::new(&id);
            log.log_with_source(
                LogLevel::Info,
                "session",
                format!("Session imported | id={}", id),
            );
        }
        Ok(count)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
