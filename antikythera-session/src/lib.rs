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
//! ├── export.rs        # Export/import with Postcard serialization
//! └── ffi.rs           # FFI bindings for SDK exposure
//! ```
//!
//! ## Usage
//!
//! ### Basic Session Management
//!
//! ```rust
//! use antikythera_session::{SessionManager, Message, MessageRole};
//!
//! let manager = SessionManager::new();
//!
//! // Create session
//! let session_id = manager.create_session("user-123", "gpt-4");
//!
//! // Add messages
//! manager.add_message(&session_id, Message::user("What's the weather?"));
//! manager.add_message(&session_id, Message::assistant("It's 72°F and sunny."));
//!
//! // Get chat history
//! let history = manager.get_chat_history(&session_id);
//! ```
//!
//! ### Export/Import Sessions
//!
//! ```rust,ignore
//! use antikythera_session::{SessionManager, SessionExport};
//!
//! let manager = SessionManager::new();
//! let session_id = manager.create_session("user-123", "gpt-4");
//!
//! // Export session to binary
//! let export_data = manager.export_session(&session_id).unwrap();
//!
//! // Save to file (Postcard binary format)
//! std::fs::write("session.pc", &export_data).unwrap();
//!
//! // Import session later
//! let data = std::fs::read("session.pc").unwrap();
//! let session = SessionExport::from_export(&data).unwrap();
//!
//! // Add back to manager
//! manager.import_session(session).unwrap();
//! ```

pub mod export;
pub mod manager;
pub mod session;

pub use export::*;
pub use manager::*;
pub use session::*;
