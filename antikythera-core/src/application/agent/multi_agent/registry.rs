//! Agent Registry for Multi-Agent Orchestration
//!
//! Manages a collection of [`AgentProfile`] entries identified by their `id`
//! field.  Each profile describes a named agent role that the orchestrator
//! can instantiate.
//!
//! **Scope**: profile storage only.  Scheduling, message routing, and
//! inter-agent communication are handled at the orchestrator layer (outside
//! this module).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Agent profile descriptor.
///
/// Profiles are stored in the [`AgentRegistry`] and consulted by the
/// orchestrator when building [`AgentOptions`] for each task.
///
/// All fields added after the initial release are `#[serde(default)]` for
/// backwards-compatible deserialization from existing JSON/TOML configs.
///
/// [`AgentOptions`]: crate::application::agent::AgentOptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Unique identifier used for routing and look-ups.
    pub id: String,
    /// Display name shown in logs and CLI output.
    pub name: String,
    /// Semantic role label (e.g. `"code-reviewer"`, `"data-analyst"`).
    pub role: String,
    /// System prompt injected into the agent's context.
    ///
    /// When `None` the agent uses the client's default instructions.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Maximum reasoning steps for this agent.
    ///
    /// Per-task `max_steps` overrides this value.  Falls back to 8 when both
    /// are `None`.
    #[serde(default)]
    pub max_steps: Option<usize>,
}

/// Agent role enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    GeneralAssistant,
    CodeReviewer,
    DataAnalyst,
    Researcher,
    Custom(String),
}

impl AgentRole {
    /// Parse a role string into the corresponding [`AgentRole`] variant.
    ///
    /// Matching is case-insensitive and hyphen/underscore-tolerant.  Strings
    /// that do not match a known variant are wrapped in [`AgentRole::Custom`].
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "general_assistant" | "general" | "assistant" => AgentRole::GeneralAssistant,
            "code_reviewer" | "reviewer" => AgentRole::CodeReviewer,
            "data_analyst" | "analyst" => AgentRole::DataAnalyst,
            "researcher" | "research" => AgentRole::Researcher,
            _ => AgentRole::Custom(s.to_string()),
        }
    }
}

impl AgentProfile {
    /// Return the [`AgentRole`] enum value that corresponds to `self.role`.
    ///
    /// The string field is kept for backwards-compatible serialisation;
    /// callers that need enum-based dispatch should use this accessor.
    pub fn role_typed(&self) -> AgentRole {
        AgentRole::from_str(&self.role)
    }
}

/// Simplified synchronous memory-provider trait used by the multi-agent
/// registry layer.
///
/// For the full async persistence contract used by [`FsmAgent`] refer to
/// [`crate::application::agent::memory::MemoryProvider`].
///
/// [`FsmAgent`]: crate::application::agent::FsmAgent
pub trait SyncMemoryProvider: Send + Sync {
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
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

/// Agent Registry - manages multiple agent profiles with sandboxing
#[derive(Clone)]
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

    /// Remove a registered profile by ID, returning it if it existed.
    pub fn remove(&mut self, id: &str) -> Option<AgentProfile> {
        self.profiles.remove(id)
    }

    /// Return the number of registered profiles.
    pub fn count(&self) -> usize {
        self.profiles.len()
    }
}

impl<P> Default for AgentRegistry<P> {
    fn default() -> Self {
        Self::new()
    }
}
