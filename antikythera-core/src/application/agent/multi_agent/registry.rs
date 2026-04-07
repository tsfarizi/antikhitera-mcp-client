//! Agent Registry for Multi-Agent Orchestration
//!
//! Stub implementation - full implementation pending profile and memory modules.

use std::collections::HashMap;
use std::marker::PhantomData;
use serde::{Deserialize, Serialize};

/// Agent profile identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub role: String,
}

/// Agent role enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    GeneralAssistant,
    CodeReviewer,
    DataAnalyst,
    Researcher,
    Custom(String),
}

/// Memory provider trait
pub trait MemoryProvider: Send + Sync {
    type Error: std::fmt::Debug;

    fn save_state(&self, agent_id: &str, state: &str) -> Result<(), Self::Error>;
    fn load_state(&self, agent_id: &str) -> Result<Option<String>, Self::Error>;
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub provider: String,
}

/// Context identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContextId(String);

impl ContextId {
    pub fn new(id: String) -> Self { Self(id) }
}

/// Agent Registry - manages multiple agent profiles with sandboxing
pub struct AgentRegistry<P> {
    profiles: HashMap<String, AgentProfile>,
    _provider: PhantomData<P>,
}

impl<P> AgentRegistry<P> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            _provider: PhantomData,
        }
    }

    /// Register a new agent profile
    pub fn register(&mut self, profile: AgentProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    /// Get a registered profile
    pub fn get_profile(&self, id: &str) -> Option<&AgentProfile> {
        self.profiles.get(id)
    }

    /// List all registered profiles
    pub fn list_profiles(&self) -> Vec<&AgentProfile> {
        self.profiles.values().collect()
    }
}

impl<P> Default for AgentRegistry<P> {
    fn default() -> Self { Self::new() }
}
