use super::errors::AgentError;
use super::fsm_runner::FsmAgent;
use super::memory::AgentStateSnapshot;
use super::models::{AgentOutcome, AgentStep};
use crate::application::model_provider::ModelProvider;
use crate::logging::AgentLogger;

impl<P: ModelProvider> FsmAgent<P> {
    /// Save intermediate state during execution
    pub(super) async fn save_intermediate_state(
        &self,
        session_id: &Option<String>,
        logs: &[String],
        steps: &[AgentStep],
        state_name: &str,
    ) -> Result<(), AgentError> {
        let context_id = session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut snapshot = AgentStateSnapshot::new(context_id.clone(), "agent".into());
        snapshot.fsm_state = state_name.to_string();
        snapshot.history = logs
            .iter()
            .map(|log| antikythera_session::Message::system(log.clone()))
            .collect();
        snapshot.metadata.steps_executed = steps.len() as u32;

        self.memory
            .save_state(snapshot)
            .await
            .map_err(|e| AgentError::InvalidResponse(format!("State persistence failed: {}", e)))?;

        let log = AgentLogger::new(&context_id);
        log.debug(format!("Intermediate state saved: {}", state_name));
        Ok(())
    }

    /// Save final state
    pub(super) async fn save_state(&self, outcome: &AgentOutcome) -> Result<(), AgentError> {
        let mut snapshot = AgentStateSnapshot::new(outcome.session_id.clone(), "agent".into());
        snapshot.fsm_state = "Terminated".to_string();
        snapshot.history = outcome
            .logs
            .iter()
            .map(|log| antikythera_session::Message::system(log.clone()))
            .collect();
        snapshot.metadata.steps_executed = outcome.steps.len() as u32;

        self.memory.save_state(snapshot).await.map_err(|e| {
            AgentError::InvalidResponse(format!("Final state persistence failed: {}", e))
        })?;

        let log = AgentLogger::new(&outcome.session_id);
        log.info("Final agent state saved");
        Ok(())
    }
}
