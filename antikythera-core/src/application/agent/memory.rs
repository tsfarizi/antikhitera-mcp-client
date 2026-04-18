//! Agent State Model
//!
//! This module defines the **data model** for agent state persistence and the
//! `MemoryProvider` trait that describes the persistence contract.
//!
//! ## Design principle — storage is the host's responsibility
//!
//! The WASM component only **produces** and **consumes** serialized state:
//!
//! - To persist: call the WIT host import `save-state(session_id, state_json)`.
//!   The host decides the backend (filesystem, Redis, GCS, database, etc.).
//! - To restore: call the WIT host import `load-state(session_id)`.
//!   The host returns the bytes it previously stored.
//!
//! No concrete `MemoryProvider` implementation lives inside the WASM component.
//! Concrete backends (filesystem, cloud storage, etc.) are implemented by the
//! host application that embeds the `.wasm` binary via FFI.
//!
//! ## What lives here
//!
//! - `AgentStateSnapshot` — serializable state blob (Postcard binary format)
//! - `ConversationTurn` / `Attachment` / `StateMetadata` — sub-types
//! - `MemoryProvider` — async trait that a host-side adapter can implement
//! - `MemoryError` — error variants shared by the trait and snapshot types

use async_trait::async_trait;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current schema version for state serialization
pub const STATE_SCHEMA_VERSION: u32 = 1;

/// Unique identifier for agent context
pub type ContextId = String;

/// Agent state snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    /// Schema version for compatibility
    pub schema_version: u32,
    /// Context identifier
    pub context_id: ContextId,
    /// Agent profile ID
    pub agent_id: String,
    /// Serialized FSM state
    pub fsm_state: String,
    /// Conversation history
    pub history: Vec<ConversationTurn>,
    /// Tool execution cache
    pub tool_cache: HashMap<String, serde_json::Value>,
    /// Context variables
    pub context_vars: HashMap<String, String>,
    /// Timestamp of last update (Unix timestamp in seconds)
    pub timestamp: i64,
    /// Execution metadata
    pub metadata: StateMetadata,
}

/// Single conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub attachments: Vec<Attachment>,
}

/// Attachment in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub mime_type: String,
    pub data: String,
    pub name: Option<String>,
}

/// Execution metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Total steps executed
    pub steps_executed: u32,
    /// Total tokens used
    pub tokens_used: u32,
    /// Last error message if any
    pub last_error: Option<String>,
    /// Custom metadata key-value pairs
    pub custom: HashMap<String, String>,
}

impl AgentStateSnapshot {
    /// Create a new state snapshot
    pub fn new(context_id: ContextId, agent_id: String) -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            context_id,
            agent_id,
            fsm_state: "Idle".to_string(),
            history: Vec::new(),
            tool_cache: HashMap::new(),
            context_vars: HashMap::new(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: StateMetadata::default(),
        }
    }

    /// Check if snapshot is compatible with current schema
    pub fn is_compatible(&self) -> bool {
        self.schema_version == STATE_SCHEMA_VERSION
    }

    /// Serialize to Postcard binary format
    pub fn to_postcard(&self) -> Result<Vec<u8>, MemoryError> {
        to_allocvec(self).map_err(MemoryError::Serialization)
    }

    /// Deserialize from Postcard binary format
    pub fn from_postcard(bytes: &[u8]) -> Result<Self, MemoryError> {
        from_bytes(bytes).map_err(MemoryError::Serialization)
    }
}

/// Memory Provider trait for state persistence
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Provider name for identification
    fn name(&self) -> &str;

    /// Initialize the provider
    async fn initialize(&mut self) -> Result<(), MemoryError>;

    /// Check if provider is ready
    async fn is_ready(&self) -> bool;

    // === State Operations ===

    /// Save agent state
    async fn save_state(&self, state: AgentStateSnapshot) -> Result<(), MemoryError>;

    /// Load agent state by context ID
    async fn load_state(&self, context_id: &ContextId) -> Result<Option<AgentStateSnapshot>, MemoryError>;

    /// Update existing state
    async fn update_state(&self, state: AgentStateSnapshot) -> Result<(), MemoryError>;

    /// Delete agent state
    async fn delete_state(&self, context_id: &ContextId) -> Result<(), MemoryError>;

    /// Check if state exists
    async fn state_exists(&self, context_id: &ContextId) -> bool;

    // === Context Management ===

    /// List all context IDs for an agent
    async fn list_contexts(&self, agent_id: &str) -> Result<Vec<ContextId>, MemoryError>;

    /// Clear all contexts for an agent
    async fn clear_agent_contexts(&self, agent_id: &str) -> Result<(), MemoryError>;

    // === Lifecycle ===

    /// Shutdown the provider gracefully
    async fn shutdown(&self) -> Result<(), MemoryError>;
}

/// Memory provider errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] postcard::Error),

    #[error("State not found: {0}")]
    NotFound(ContextId),

    #[error("Schema version mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: u32, actual: u32 },

    #[error("Provider not initialized")]
    NotInitialized,

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postcard_serialization() {
        let state = AgentStateSnapshot::new("test".into(), "agent".into());

        // Serialize
        let bytes = state.to_postcard().unwrap();

        // Deserialize
        let loaded = AgentStateSnapshot::from_postcard(&bytes).unwrap();

        assert_eq!(loaded.context_id, state.context_id);
        assert_eq!(loaded.agent_id, state.agent_id);
        assert_eq!(loaded.schema_version, state.schema_version);
    }
}
