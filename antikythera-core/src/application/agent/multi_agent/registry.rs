//! Agent Registry for Multi-Agent Orchestration
//!
//! Provides a centralized registry for managing multiple agent profiles
//! with strict sandboxing and context isolation.

use super::profile::{AgentProfile, AgentProfileError, DEFAULT_AGENT_ID};
use super::memory::{MemoryProvider, InMemoryMemory, MemoryConfig, ContextId, AgentStateSnapshot, ConversationTurn};
use crate::application::agent::{Agent, AgentOptions, AgentOutcome};
use crate::application::client::McpClient;
use crate::infrastructure::model::ModelProvider;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, instrument};

/// Agent Registry - manages multiple agent profiles with sandboxing
pub struct AgentRegistry<P: ModelProvider> {
    /// Registered agent profiles
    profiles: RwLock<HashMap<String, AgentProfile>>,
    /// Memory provider for state persistence
    memory: Arc<dyn MemoryProvider<Error = MemoryError>>,
    /// Active agent contexts (for tracking concurrent executions)
    active_contexts: RwLock<HashMap<ContextId, ActiveAgentContext>>,
    /// Default client for agent creation
    _client: Arc<McpClient<P>>,
    /// Phantom marker for provider type
    _provider: std::marker::PhantomData<P>,
}

/// Active agent context tracking
#[derive(Debug, Clone)]
struct ActiveAgentContext {
    agent_id: String,
    created_at: i64,
    last_activity: i64,
}

/// Memory provider error wrapper
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Memory operation failed: {0}")]
    OperationFailed(String),

    #[error("Context not found: {0}")]
    ContextNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Agent execution error: {0}")]
    AgentExecutionError(String),
}

impl From<InMemoryMemory> for MemoryError {
    fn from(err: InMemoryMemory) -> Self {
        MemoryError::OperationFailed(err.to_string())
    }
}

impl<P: ModelProvider> AgentRegistry<P> {
    /// Create a new agent registry with default in-memory storage
    pub fn new(client: Arc<McpClient<P>>) -> Self {
        let memory = Arc::new(InMemoryMemory::default());
        
        Self {
            profiles: RwLock::new(HashMap::new()),
            memory,
            active_contexts: RwLock::new(HashMap::new()),
            _client: client,
            _provider: std::marker::PhantomData,
        }
    }

    /// Create a new agent registry with custom memory provider
    pub fn with_memory(
        client: Arc<McpClient<P>>,
        memory: Arc<dyn MemoryProvider<Error = MemoryError>>,
    ) -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            memory,
            active_contexts: RwLock::new(HashMap::new()),
            _client: client,
            _provider: std::marker::PhantomData,
        }
    }

    /// Initialize the registry (loads default profile)
    pub async fn initialize(&self) -> Result<(), MemoryError> {
        // Register default profile for backward compatibility
        let default_profile = AgentProfile::default_profile();
        self.register_profile(default_profile).await?;
        
        // Initialize memory provider
        // Note: Can't mutate self.memory, so we assume it's already initialized
        // or initialize it before passing to the registry
        
        info!("Agent registry initialized with default profile");
        Ok(())
    }

    // === Profile Management ===

    /// Register an agent profile
    #[instrument(skip(self), fields(profile_id))]
    pub async fn register_profile(&self, profile: AgentProfile) -> Result<(), MemoryError> {
        profile.validate().map_err(|e| {
            MemoryError::OperationFailed(format!("Profile validation failed: {}", e))
        })?;

        tracing::Span::current().record("profile_id", &profile.id);
        
        let mut profiles = self.profiles.write().await;
        profiles.insert(profile.id.clone(), profile);
        
        info!("Registered agent profile");
        Ok(())
    }

    /// Get a profile by ID
    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<AgentProfile>, MemoryError> {
        let profiles = self.profiles.read().await;
        Ok(profiles.get(profile_id).cloned())
    }

    /// List all registered profiles
    pub async fn list_profiles(&self) -> Result<Vec<AgentProfile>, MemoryError> {
        let profiles = self.profiles.read().await;
        Ok(profiles.values().cloned().collect())
    }

    /// Update a profile
    #[instrument(skip(self), fields(profile_id))]
    pub async fn update_profile(&self, profile: AgentProfile) -> Result<(), MemoryError> {
        tracing::Span::current().record("profile_id", &profile.id);
        
        profile.validate().map_err(|e| {
            MemoryError::OperationFailed(format!("Profile validation failed: {}", e))
        })?;

        let mut profiles = self.profiles.write().await;
        if !profiles.contains_key(&profile.id) {
            return Err(MemoryError::ProfileNotFound(profile.id.clone()));
        }
        
        profiles.insert(profile.id.clone(), profile);
        info!("Updated agent profile");
        Ok(())
    }

    /// Delete a profile (except default)
    #[instrument(skip(self), fields(profile_id))]
    pub async fn delete_profile(&self, profile_id: &str) -> Result<(), MemoryError> {
        tracing::Span::current().record("profile_id", profile_id);
        
        if profile_id == DEFAULT_AGENT_ID {
            return Err(MemoryError::OperationFailed(
                "Cannot delete default profile".into(),
            ));
        }

        let mut profiles = self.profiles.write().await;
        if profiles.remove(profile_id).is_none() {
            return Err(MemoryError::ProfileNotFound(profile_id.to_string()));
        }
        
        info!("Deleted agent profile");
        Ok(())
    }

    // === Agent Execution with Sandboxing ===

    /// Execute an agent with a specific profile
    #[instrument(skip(self, prompt, options), fields(profile_id, context_id))]
    pub async fn execute_agent(
        &self,
        profile_id: &str,
        prompt: String,
        mut options: AgentOptions,
    ) -> Result<AgentOutcome, MemoryError> {
        // Get profile
        let profile = self.get_profile(profile_id).await?
            .ok_or_else(|| MemoryError::ProfileNotFound(profile_id.to_string()))?;

        tracing::Span::current().record("profile_id", &profile.id);

        // Create unique context ID for sandboxing
        let context_id = self.memory.create_context_id();
        tracing::Span::current().record("context_id", &context_id);

        // Track active context
        let active_context = ActiveAgentContext {
            agent_id: profile.id.clone(),
            created_at: chrono::Utc::now().timestamp(),
            last_activity: chrono::Utc::now().timestamp(),
        };
        self.active_contexts.write().await.insert(context_id.clone(), active_context);

        // Set session ID from context
        options.session_id = Some(context_id.clone());

        // Override system prompt with profile template
        options.system_prompt = Some(profile.system_prompt());

        info!(
            profile_id = %profile.id,
            context_id = %context_id,
            "Executing agent with profile"
        );

        // Execute agent (sandboxed - each execution has unique context)
        // Note: In a real implementation, you'd create a new Agent instance
        // with the profile-specific configuration
        let outcome = self.execute_with_profile(&profile, context_id.clone(), prompt, options).await?;

        // Update active context timestamp
        if let Some(ctx) = self.active_contexts.write().await.get_mut(&context_id) {
            ctx.last_activity = chrono::Utc::now().timestamp();
        }

        Ok(outcome)
    }

    /// Internal execution with profile-specific configuration
    async fn execute_with_profile(
        &self,
        profile: &AgentProfile,
        context_id: ContextId,
        prompt: String,
        options: AgentOptions,
    ) -> Result<AgentOutcome, MemoryError> {
        // This is a simplified implementation
        // In production, you would:
        // 1. Create an Agent instance with profile-specific tool filtering
        // 2. Load previous state from memory if exists
        // 3. Execute the agent loop
        // 4. Save state to memory after execution

        // For now, we'll use the default agent execution
        // The actual implementation would be in the CLI crate where Agent is accessible
        
        // Load previous state if exists
        let previous_state = self.memory.load_state(&context_id).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?;

        if let Some(state) = previous_state {
            info!(
                context_id = %context_id,
                history_len = state.history.len(),
                "Resumed agent from previous state"
            );
        }

        // Execute agent (placeholder - actual implementation in CLI)
        // This would call the actual Agent::run() with profile-specific configuration
        let outcome = AgentOutcome {
            logs: vec!["Profile-based execution (placeholder)".into()],
            session_id: context_id,
            response: format!("Executed with profile: {}", profile.name),
            steps: vec![],
        };

        // Save state after execution
        let snapshot = AgentStateSnapshot {
            context_id: outcome.session_id.clone(),
            agent_id: profile.id.clone(),
            fsm_state: "completed".into(),
            history: vec![],
            tool_cache: HashMap::new(),
            context_vars: profile.context_vars.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.memory.save_state(snapshot).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?;

        Ok(outcome)
    }

    // === Context Management ===

    /// Get context history
    pub async fn get_context_history(
        &self,
        context_id: &ContextId,
    ) -> Result<Vec<ConversationTurn>, MemoryError> {
        self.memory.get_history(context_id, None).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))
    }

    /// Delete a context
    pub async fn delete_context(&self, context_id: &ContextId) -> Result<(), MemoryError> {
        self.memory.delete_state(context_id).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?;
        
        self.active_contexts.write().await.remove(context_id);
        
        info!(context_id = %context_id, "Deleted context");
        Ok(())
    }

    /// List active contexts
    pub async fn list_active_contexts(&self) -> Result<Vec<ContextId>, MemoryError> {
        let contexts = self.active_contexts.read().await;
        Ok(contexts.keys().cloned().collect())
    }

    /// Get active contexts for a specific profile
    pub async fn list_profile_contexts(&self, profile_id: &str) -> Result<Vec<ContextId>, MemoryError> {
        let contexts = self.active_contexts.read().await;
        Ok(contexts
            .iter()
            .filter(|(_, ctx)| ctx.agent_id == profile_id)
            .map(|(id, _)| id.clone())
            .collect())
    }

    // === Memory Operations ===

    /// Get the memory provider
    pub fn memory(&self) -> Arc<dyn MemoryProvider<Error = MemoryError>> {
        self.memory.clone()
    }

    /// Pause agent state (serialize to memory)
    #[instrument(skip(self), fields(context_id))]
    pub async fn pause_agent(&self, context_id: &ContextId) -> Result<(), MemoryError> {
        tracing::Span::current().record("context_id", context_id);
        
        // Load current state
        let mut state = self.memory.load_state(context_id).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?
            .ok_or_else(|| MemoryError::ContextNotFound(context_id.clone()))?;

        // Update FSM state to "paused"
        state.fsm_state = "paused".into();
        state.timestamp = chrono::Utc::now().timestamp();

        // Save paused state
        self.memory.save_state(state).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?;

        info!("Paused agent state");
        Ok(())
    }

    /// Resume agent state (deserialize from memory)
    #[instrument(skip(self), fields(context_id))]
    pub async fn resume_agent(&self, context_id: &ContextId) -> Result<AgentStateSnapshot, MemoryError> {
        tracing::Span::current().record("context_id", context_id);
        
        let state = self.memory.load_state(context_id).await
            .map_err(|e| MemoryError::OperationFailed(e.to_string()))?
            .ok_or_else(|| MemoryError::ContextNotFound(context_id.clone()))?;

        if state.fsm_state != "paused" {
            warn!(
                context_id = %context_id,
                state = %state.fsm_state,
                "Resuming agent that was not paused"
            );
        }

        info!("Resumed agent state");
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_initialization() {
        // Create a mock client (would need McpClient mock in real tests)
        // For now, just test the registry structure
        
        let memory = Arc::new(InMemoryMemory::default());
        assert!(memory.is_ready().await);
    }

    #[tokio::test]
    async fn test_profile_registration() {
        use super::profile::{AgentProfile, AgentRole};
        
        let profile = AgentProfile::new("Test Agent".into(), AgentRole::General);
        assert!(profile.validate().is_ok());
        assert_eq!(profile.name, "Test Agent");
        assert_eq!(profile.role, AgentRole::General);
    }

    #[tokio::test]
    async fn test_tool_access_policy() {
        use super::profile::ToolAccessPolicy;
        use std::collections::HashSet;

        // Test Allowlist
        let mut policy = ToolAccessPolicy::AllowAll;
        policy.allow_tool("tool1");
        assert!(matches!(policy, ToolAccessPolicy::Allowlist(_)));
        
        if let ToolAccessPolicy::Allowlist(allowed) = &policy {
            assert!(allowed.contains("tool1"));
        }

        // Test Denylist
        let mut policy = ToolAccessPolicy::DenyAll;
        policy.deny_tool("tool2");
        assert!(matches!(policy, ToolAccessPolicy::Denylist(_)));
    }
}
