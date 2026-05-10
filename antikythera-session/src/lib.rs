//! # Antikythera Session
//!
//! Session management for Antikythera MCP Framework with persistent chat history.
//!
//! ## Features
//!
//! - **Concurrent sessions** - Multiple sessions managed simultaneously
//! - **Parallel operations** - Thread-safe session operations
//! - **Persistent state** - Export/import sessions with consistent format
//! - **Chat history** - Full tracking of user, assistant, and tool interactions
//! - **Typed consistency** - Rust type system ensures valid session data
//! - **Postcard serialization** - Efficient binary format for persistence
//!
//! ## Architecture
//!
//! ```text
//! antikythera-session/
//! ├── session.rs       # Session entity with chat history
//! ├── manager.rs       # Session manager (concurrent access)
//! └── export.rs        # Export/import with Postcard serialization
//! ```
//!
//! ## Usage
//!
//! ### Basic Session Management
//!
//! ```rust
//! use antikythera_session::{SessionManager, Message};
//!
//! let manager = SessionManager::new();
//!
//! // Create session
//! let session_id = manager.create_session("user-123", "gpt-4").unwrap();
//!
//! // Add messages
//! manager.add_message(&session_id, Message::user("What's the weather?")).unwrap();
//! manager.add_message(&session_id, Message::assistant("It's 72°F and sunny.")).unwrap();
//!
//! // Get chat history
//! let history = manager.get_chat_history(&session_id).unwrap();
//! ```
//!
//! ### Export/Import Sessions
//!
//! ```rust,ignore
//! use antikythera_session::{SessionManager, SessionExport};
//!
//! let manager = SessionManager::new();
//! let session_id = manager.create_session("user-123", "gpt-4").unwrap();
//!
//! // Export session to binary
//! let session = manager.get_session(&session_id).unwrap().unwrap();
//! let export = SessionExport::from_session(session);
//! let data = export.to_postcard().unwrap();
//!
//! // Import session later
//! let export = SessionExport::from_postcard(&data).unwrap();
//! manager.import_session(export.into_session()).unwrap();
//! ```

pub mod export;
pub mod manager;
pub mod session;

pub use export::*;
pub use manager::*;
pub use session::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_create_and_get() {
        let manager = SessionManager::new();
        let id = manager.create_session("user-1", "gpt-4").unwrap();
        assert!(!id.is_empty());

        let session = manager.get_session(&id).unwrap();
        assert!(session.is_some());
        let s = session.unwrap();
        assert_eq!(s.user_id, "user-1");
        assert_eq!(s.model, "gpt-4");
    }

    #[test]
    fn session_manager_delete() {
        let manager = SessionManager::new();
        let id = manager.create_session("user-1", "gpt-4").unwrap();
        assert!(manager.has_session(&id).unwrap());

        manager.delete_session(&id).unwrap();
        assert!(!manager.has_session(&id).unwrap());
    }

    #[test]
    fn session_manager_session_not_found() {
        let manager = SessionManager::new();
        let err = manager.get_session("nonexistent").unwrap();
        assert!(err.is_none());
    }

    #[test]
    fn session_manager_error_partial_eq() {
        let a = SessionManagerError::SessionNotFound("a".to_string());
        let b = SessionManagerError::SessionNotFound("a".to_string());
        let c = SessionManagerError::SessionNotFound("b".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn session_manager_add_and_get_messages() {
        let manager = SessionManager::new();
        let id = manager.create_session("user-1", "gpt-4").unwrap();

        manager.add_message(&id, Message::user("hello")).unwrap();
        manager
            .add_message(&id, Message::assistant("hi there"))
            .unwrap();

        let history = manager.get_chat_history(&id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "hello");
        assert_eq!(history[1].content, "hi there");
    }

    #[test]
    fn session_manager_list_and_count() {
        let manager = SessionManager::new();
        manager.create_session("u1", "m1").unwrap();
        manager.create_session("u2", "m2").unwrap();

        let count = manager.session_count().unwrap();
        assert_eq!(count, 2);

        let list = manager.list_sessions().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn session_export_roundtrip() {
        let mut session = Session::new("user-1", "gpt-4");
        session.add_message(Message::user("hello"));

        let export = SessionExport::from_session(session.clone());
        let data = export.to_postcard().unwrap();

        let restored_export = SessionExport::from_postcard(&data).unwrap();
        let restored = restored_export.into_session();

        assert_eq!(restored.id, session.id);
        assert_eq!(restored.messages.len(), 1);
        assert_eq!(restored.messages[0].content, "hello");
    }

    #[test]
    fn message_role_serialization() {
        let msg = Message::user("test");
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.role, msg.role);
        assert_eq!(restored.content, "test");
    }
}
