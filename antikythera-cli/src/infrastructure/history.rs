//! Debug Chat History Store
//!
//! Captures every chat exchange during a TUI session and persists it as JSON
//! files under `debug/chat_history/<uuid>.json` (relative to the working
//! directory, same location as `app.pc`).
//!
//! This is a **CLI-local, debug-only feature**.  The core protocol layer is
//! unaware of it.  History is written after each successful assistant turn so
//! partial sessions are never lost.
//!
//! ## Storage format
//! Each session is a single pretty-printed JSON file whose name is the session
//! UUID.  The file is overwritten on every turn to stay up-to-date.
//!
//! ## CRUD surface
//! | Operation | How                                            |
//! |-----------|------------------------------------------------|
//! | Create    | Automatic — first user message in a new session |
//! | Read      | `list_sessions()`, `load_session(id)`           |
//! | Update    | `rename_session(id, new_title)`                 |
//! | Delete    | `delete_session(id)`                            |

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Default storage directory relative to the working directory.
pub const HISTORY_DIR: &str = "debug/chat_history";

// ── Data types ────────────────────────────────────────────────────────────────

/// Speaker of a chat turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnRole {
    User,
    Assistant,
}

/// One message in a chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    /// ISO-8601 timestamp of when the turn was captured.
    pub timestamp: String,
    pub role: TurnRole,
    pub content: String,
    /// Number of agent tool-call steps executed to produce this turn (0 for
    /// plain chat responses).
    #[serde(default)]
    pub tool_steps: usize,
}

/// Complete record of one debug chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistorySession {
    /// Local UUID used as the filename key.
    pub id: String,
    /// ISO-8601 timestamp of session creation.
    pub created_at: String,
    /// ISO-8601 timestamp of last write — used for sort order.
    pub updated_at: String,
    /// Display title (first 60 chars of the first user message by default).
    /// Can be renamed by the user.
    pub title: String,
    pub provider: String,
    pub model: String,
    pub agent_mode: bool,
    /// The core-side session ID that ties this to the provider context window.
    pub core_session_id: Option<String>,
    pub turns: Vec<ChatTurn>,
}

impl ChatHistorySession {
    /// Create an empty session.  Call [`ChatHistoryStore::save_session`] after
    /// adding the first turn.
    pub fn new(id: String, provider: String, model: String, agent_mode: bool) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id,
            created_at: now.clone(),
            updated_at: now,
            title: String::new(),
            provider,
            model,
            agent_mode,
            core_session_id: None,
            turns: Vec::new(),
        }
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Lightweight handle for the on-disk session store.
///
/// All operations are synchronous (no Tokio overhead) since they run on the
/// event-loop tick, which is fine for small debug JSON files.
pub struct ChatHistoryStore {
    dir: PathBuf,
}

impl ChatHistoryStore {
    /// Construct with the default storage directory.
    pub fn new() -> Self {
        Self {
            dir: PathBuf::from(HISTORY_DIR),
        }
    }

    /// Construct with a custom storage directory (useful in tests).
    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Generate a fresh session UUID.
    pub fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    // ── Internals ─────────────────────────────────────────────────────────

    fn session_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }

    // ── CRUD ──────────────────────────────────────────────────────────────

    /// Persist (create or overwrite) a session to disk.
    pub fn save_session(&self, session: &ChatHistorySession) -> Result<(), String> {
        self.ensure_dir().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
        fs::write(self.session_path(&session.id), json).map_err(|e| e.to_string())
    }

    /// Return all sessions sorted by `updated_at` descending (most recent first).
    ///
    /// Files that fail to parse are silently skipped — this keeps the browser
    /// functional even if a file is manually corrupted.
    pub fn list_sessions(&self) -> Vec<ChatHistorySession> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut sessions: Vec<ChatHistorySession> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .filter_map(|e| {
                let content = fs::read_to_string(e.path()).ok()?;
                serde_json::from_str(&content).ok()
            })
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    /// Load a single session by its local UUID.  Returns `None` if the file
    /// does not exist or cannot be parsed.
    pub fn load_session(&self, id: &str) -> Option<ChatHistorySession> {
        let content = fs::read_to_string(self.session_path(id)).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Permanently remove a session file.
    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        fs::remove_file(self.session_path(id)).map_err(|e| e.to_string())
    }

    /// Update only the display title, preserving all other fields.
    pub fn rename_session(&self, id: &str, new_title: String) -> Result<(), String> {
        let mut session = self
            .load_session(id)
            .ok_or_else(|| format!("Sesi '{id}' tidak ditemukan."))?;
        session.title = new_title;
        session.updated_at = Utc::now().to_rfc3339();
        self.save_session(&session)
    }
}

impl Default for ChatHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}
