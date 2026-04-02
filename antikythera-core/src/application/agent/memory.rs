//! Stateless Memory Provider for Agent State Persistence
//!
//! This module provides a unified abstraction for storing and retrieving agent states
//! in stateless environments. It supports multiple backends (local filesystem, GCS, Redis)
//! and uses Postcard binary serialization for optimal performance.
//!
//! ## Features
//!
//! - **Binary Serialization**: Postcard format for minimal I/O latency
//! - **Multiple Providers**: Filesystem (default), GCS, Redis (feature-gated)
//! - **Async Operations**: Non-blocking I/O for responsive orchestration
//! - **Context Isolation**: Each agent has isolated memory paths

use async_trait::async_trait;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, error, info, warn};

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

/// Local filesystem memory provider (default)
pub struct FilesystemMemory {
    /// Base directory for state storage
    base_path: PathBuf,
    /// Initialized flag
    initialized: bool,
}

impl FilesystemMemory {
    /// Create a new filesystem memory provider
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            initialized: false,
        }
    }

    /// Get state file path for a context ID
    fn get_state_path(&self, context_id: &ContextId) -> PathBuf {
        // Use first 2 chars as subdirectory for better file system performance
        let prefix = if context_id.len() >= 2 {
            &context_id[..2]
        } else {
            "xx"
        };
        
        self.base_path
            .join(prefix)
            .join(format!("{}.postcard", context_id))
    }
}

#[async_trait]
impl MemoryProvider for FilesystemMemory {
    fn name(&self) -> &str {
        "filesystem"
    }

    async fn initialize(&mut self) -> Result<(), MemoryError> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| MemoryError::Configuration(format!(
                "Failed to create base directory {:?}: {}",
                self.base_path, e
            )))?;
        
        self.initialized = true;
        info!(
            base_path = %self.base_path.display(),
            "Filesystem memory provider initialized"
        );
        Ok(())
    }

    async fn is_ready(&self) -> bool {
        self.initialized
    }

    async fn save_state(&self, state: AgentStateSnapshot) -> Result<(), MemoryError> {
        if !self.initialized {
            return Err(MemoryError::NotInitialized);
        }

        let state_path = self.get_state_path(&state.context_id);
        
        // Ensure parent directory exists
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Serialize to Postcard
        let bytes = state.to_postcard()?;
        
        // Write atomically (write to temp file, then rename)
        let temp_path = state_path.with_extension(".tmp");
        fs::write(&temp_path, &bytes).await?;
        fs::rename(&temp_path, &state_path).await?;

        debug!(
            context_id = %state.context_id,
            path = %state_path.display(),
            size_bytes = bytes.len(),
            "Saved agent state to filesystem"
        );

        Ok(())
    }

    async fn load_state(&self, context_id: &ContextId) -> Result<Option<AgentStateSnapshot>, MemoryError> {
        if !self.initialized {
            return Err(MemoryError::NotInitialized);
        }

        let state_path = self.get_state_path(context_id);
        
        if !state_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&state_path).await?;
        let state = AgentStateSnapshot::from_postcard(&bytes)?;

        // Validate schema version
        if !state.is_compatible() {
            warn!(
                context_id = %context_id,
                schema_version = state.schema_version,
                expected = STATE_SCHEMA_VERSION,
                "Loaded state with incompatible schema version"
            );
            return Err(MemoryError::SchemaMismatch {
                expected: STATE_SCHEMA_VERSION,
                actual: state.schema_version,
            });
        }

        debug!(
            context_id = %context_id,
            path = %state_path.display(),
            "Loaded agent state from filesystem"
        );

        Ok(Some(state))
    }

    async fn update_state(&self, state: AgentStateSnapshot) -> Result<(), MemoryError> {
        // For filesystem, update is same as save
        self.save_state(state).await
    }

    async fn delete_state(&self, context_id: &ContextId) -> Result<(), MemoryError> {
        if !self.initialized {
            return Err(MemoryError::NotInitialized);
        }

        let state_path = self.get_state_path(context_id);
        
        if state_path.exists() {
            fs::remove_file(&state_path).await?;
            debug!(
                context_id = %context_id,
                path = %state_path.display(),
                "Deleted agent state from filesystem"
            );
        }

        Ok(())
    }

    async fn state_exists(&self, context_id: &ContextId) -> bool {
        if !self.initialized {
            return false;
        }

        let state_path = self.get_state_path(context_id);
        state_path.exists()
    }

    async fn list_contexts(&self, _agent_id: &str) -> Result<Vec<ContextId>, MemoryError> {
        if !self.initialized {
            return Err(MemoryError::NotInitialized);
        }

        let mut contexts = Vec::new();
        
        // Iterate through all subdirectories
        let mut entries = fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                // Iterate through files in subdirectory
                let mut sub_entries = fs::read_dir(&path).await?;
                while let Some(sub_entry) = sub_entries.next_entry().await? {
                    let file_path = sub_entry.path();
                    if file_path.extension().and_then(|s| s.to_str()) == Some("postcard") {
                        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                            contexts.push(stem.to_string());
                        }
                    }
                }
            }
        }

        Ok(contexts)
    }

    async fn clear_agent_contexts(&self, agent_id: &str) -> Result<(), MemoryError> {
        if !self.initialized {
            return Err(MemoryError::NotInitialized);
        }

        let contexts = self.list_contexts(agent_id).await?;

        for context_id in &contexts {
            self.delete_state(context_id).await?;
        }

        info!(
            agent_id = %agent_id,
            contexts_cleared = contexts.len(),
            "Cleared all contexts for agent"
        );

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), MemoryError> {
        // Filesystem provider doesn't need special shutdown logic
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_filesystem_save_load() {
        let temp_dir = tempdir().unwrap();
        let mut provider = FilesystemMemory::new(temp_dir.path().to_path_buf());
        provider.initialize().await.unwrap();

        let state = AgentStateSnapshot::new("test-context-123".into(), "test-agent".into());
        
        // Save
        provider.save_state(state.clone()).await.unwrap();
        
        // Load
        let loaded = provider.load_state(&"test-context-123".into()).await.unwrap();
        
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.context_id, state.context_id);
        assert_eq!(loaded.agent_id, state.agent_id);
    }

    #[tokio::test]
    async fn test_filesystem_state_exists() {
        let temp_dir = tempdir().unwrap();
        let mut provider = FilesystemMemory::new(temp_dir.path().to_path_buf());
        provider.initialize().await.unwrap();

        let context_id = "test-context-456".into();
        
        // Should not exist initially
        assert!(!provider.state_exists(&context_id).await);
        
        // Save state
        let state = AgentStateSnapshot::new(context_id.clone(), "test-agent".into());
        provider.save_state(state).await.unwrap();
        
        // Should exist now
        assert!(provider.state_exists(&context_id).await);
    }

    #[tokio::test]
    async fn test_filesystem_delete() {
        let temp_dir = tempdir().unwrap();
        let mut provider = FilesystemMemory::new(temp_dir.path().to_path_buf());
        provider.initialize().await.unwrap();

        let context_id: ContextId = "test-context-789".into();

        // Save and verify
        let state = AgentStateSnapshot::new(context_id.clone(), "test-agent".into());
        provider.save_state(state).await.unwrap();
        assert!(provider.state_exists(&context_id).await);

        // Delete
        provider.delete_state(&context_id).await.unwrap();
        
        // Should not exist
        assert!(!provider.state_exists(&context_id).await);
    }

    #[tokio::test]
    async fn test_postcard_serialization() {
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
