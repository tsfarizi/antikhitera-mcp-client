//! Session Export/Import
//!
//! Export and import sessions with consistent Postcard binary format.

use crate::session::Session;
use serde::{Deserialize, Serialize};

// ============================================================================
/// Export Format
// ============================================================================

/// Session export data with versioning for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    /// Export format version
    pub version: u32,
    /// Session data
    pub session: Session,
    /// Export timestamp
    pub exported_at: String,
    /// Optional notes
    pub notes: Option<String>,
}

impl SessionExport {
    /// Current export format version
    pub const VERSION: u32 = 1;

    /// Create export from session
    pub fn from_session(session: Session) -> Self {
        Self {
            version: Self::VERSION,
            exported_at: chrono::Utc::now().to_rfc3339(),
            session,
            notes: None,
        }
    }

    /// Create export with notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Get the session
    pub fn into_session(self) -> Session {
        self.session
    }

    /// Serialize to Postcard binary
    pub fn to_postcard(&self) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from Postcard binary
    pub fn from_postcard(data: &[u8]) -> Result<Self, String> {
        let export: SessionExport = postcard::from_bytes(data)
            .map_err(|e| format!("Deserialize error: {}", e))?;

        // Validate version
        if export.version != Self::VERSION {
            return Err(format!(
                "Unsupported export version: {}. Expected: {}",
                export.version,
                Self::VERSION
            ));
        }

        Ok(export)
    }

    /// Serialize to JSON (for debugging/inspection)
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}

// ============================================================================
/// Batch Export/Import
// ============================================================================

/// Multiple sessions export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExport {
    /// Export format version
    pub version: u32,
    /// Exported sessions
    pub sessions: Vec<SessionExport>,
    /// Export timestamp
    pub exported_at: String,
    /// Optional notes
    pub notes: Option<String>,
}

impl BatchExport {
    /// Current batch export format version
    pub const VERSION: u32 = 1;

    /// Create batch export from sessions
    pub fn from_sessions(sessions: Vec<Session>) -> Self {
        Self {
            version: Self::VERSION,
            exported_at: chrono::Utc::now().to_rfc3339(),
            sessions: sessions
                .into_iter()
                .map(SessionExport::from_session)
                .collect(),
            notes: None,
        }
    }

    /// Create with notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get sessions
    pub fn into_sessions(self) -> Vec<Session> {
        self.sessions.into_iter().map(|e| e.into_session()).collect()
    }

    /// Serialize to Postcard binary
    pub fn to_postcard(&self) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from Postcard binary
    pub fn from_postcard(data: &[u8]) -> Result<Self, String> {
        let export: BatchExport = postcard::from_bytes(data)
            .map_err(|e| format!("Deserialize error: {}", e))?;

        // Validate version
        if export.version != Self::VERSION {
            return Err(format!(
                "Unsupported export version: {}. Expected: {}",
                export.version,
                Self::VERSION
            ));
        }

        Ok(export)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}
