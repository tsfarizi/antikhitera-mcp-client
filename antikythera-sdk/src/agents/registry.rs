//! Agent Registry Implementation

use super::types::{AgentConfig, AgentStatus, AgentValidationResult};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: Mutex<HashMap<String, AgentConfig>>,
    statuses: Mutex<HashMap<String, AgentStatus>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn default_status(config: &AgentConfig) -> AgentStatus {
        AgentStatus {
            id: config.id.clone(),
            name: config.name.clone(),
            active: false,
            session_id: None,
            tasks_completed: 0,
            tasks_failed: 0,
        }
    }

    pub fn register(&self, config: AgentConfig) -> AgentValidationResult {
        let validation = config.validate();
        if !validation.valid {
            return validation;
        }

        let id = config.id.clone();
        let mut agents = match self.agents.lock() {
            Ok(agents) => agents,
            Err(_) => {
                return AgentValidationResult {
                    valid: false,
                    errors: vec!["Failed to lock agent registry".to_string()],
                    agent_id: id,
                };
            }
        };

        if agents.contains_key(&id) {
            return AgentValidationResult {
                valid: false,
                errors: vec![format!("Agent '{}' already exists", id)],
                agent_id: id,
            };
        }

        let mut statuses = match self.statuses.lock() {
            Ok(statuses) => statuses,
            Err(_) => {
                return AgentValidationResult {
                    valid: false,
                    errors: vec!["Failed to lock agent status registry".to_string()],
                    agent_id: id,
                };
            }
        };

        statuses.insert(id.clone(), Self::default_status(&config));
        agents.insert(id.clone(), config);

        AgentValidationResult {
            valid: true,
            errors: Vec::new(),
            agent_id: id,
        }
    }

    pub fn unregister(&self, id: &str) -> Result<bool, String> {
        let mut agents = self
            .agents
            .lock()
            .map_err(|_| "Failed to lock agent registry".to_string())?;
        let removed = agents.remove(id).is_some();

        if removed {
            let mut statuses = self
                .statuses
                .lock()
                .map_err(|_| "Failed to lock agent status registry".to_string())?;
            statuses.remove(id);
        }

        Ok(removed)
    }

    pub fn get(&self, id: &str) -> Result<Option<AgentConfig>, String> {
        let agents = self
            .agents
            .lock()
            .map_err(|_| "Failed to lock agent registry".to_string())?;
        Ok(agents.get(id).cloned())
    }

    pub fn list(&self) -> Result<Vec<AgentConfig>, String> {
        let agents = self
            .agents
            .lock()
            .map_err(|_| "Failed to lock agent registry".to_string())?;
        Ok(agents.values().cloned().collect())
    }

    pub fn status_list(&self) -> Result<Vec<AgentStatus>, String> {
        let statuses = self
            .statuses
            .lock()
            .map_err(|_| "Failed to lock agent status registry".to_string())?;
        Ok(statuses.values().cloned().collect())
    }

    pub fn export_json(&self) -> Result<String, String> {
        let agents = self.list()?;
        serde_json::to_string(&agents).map_err(|e| format!("Failed to serialize agents: {e}"))
    }

    pub fn import_json(&self, config_json: &str) -> Result<usize, String> {
        let configs: Vec<AgentConfig> =
            serde_json::from_str(config_json).map_err(|e| format!("Invalid JSON: {e}"))?;

        let mut agents = self
            .agents
            .lock()
            .map_err(|_| "Failed to lock agent registry".to_string())?;
        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "Failed to lock agent status registry".to_string())?;

        for config in &configs {
            statuses.insert(config.id.clone(), Self::default_status(config));
            agents.insert(config.id.clone(), config.clone());
        }

        Ok(configs.len())
    }
}

static GLOBAL_AGENT_REGISTRY: LazyLock<AgentRegistry> = LazyLock::new(AgentRegistry::new);

pub fn global_agent_registry() -> &'static AgentRegistry {
    &GLOBAL_AGENT_REGISTRY
}
