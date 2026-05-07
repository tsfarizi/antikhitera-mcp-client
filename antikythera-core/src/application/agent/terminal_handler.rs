use super::errors::AgentError;
use super::fsm_runner::FsmAgent;
use super::models::{AgentOutcome, AgentStep};
use super::state::{AgentState, TerminationReason};
use crate::application::model_provider::ModelProvider;
use crate::logging::AgentLogger;
use serde_json::Value;

impl<P: ModelProvider> FsmAgent<P> {
    /// Handle terminal state and return outcome
    pub(super) fn handle_terminal_state(
        &self,
        state: AgentState,
        session_id: Option<String>,
        logs: Vec<String>,
        steps: Vec<AgentStep>,
    ) -> Result<AgentOutcome, AgentError> {
        let log = AgentLogger::new(
            session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );
        match state {
            AgentState::FinalMessage {
                content,
                data,
                metadata,
            } => {
                log.info("Agent reached FinalMessage state with structured response");

                let mut response_obj = serde_json::Map::new();
                response_obj.insert(
                    "content".to_string(),
                    serde_json::Value::String(content.clone()),
                );

                if let Some(data_value) = data {
                    response_obj.insert("data".to_string(), data_value);
                }

                if let Some(metadata_value) = metadata {
                    response_obj.insert("metadata".to_string(), metadata_value);
                }

                let structured_response = Value::Object(response_obj);

                Ok(AgentOutcome {
                    logs,
                    session_id: session_id.unwrap_or_default(),
                    response: structured_response,
                    steps,
                })
            }
            AgentState::Terminated { reason } => match reason {
                TerminationReason::Success if !steps.is_empty() => {
                    let last_step = steps.last().expect(
                        "AgentState Terminated::Success with empty steps — invariant violation",
                    );
                    Ok(AgentOutcome {
                        logs,
                        session_id: session_id.unwrap_or_default(),
                        response: Value::String(last_step.message.clone().unwrap_or_default()),
                        steps,
                    })
                }
                TerminationReason::Error { message } => Err(AgentError::InvalidResponse(message)),
                TerminationReason::MaxStepsExceeded => Err(AgentError::MaxStepsExceeded),
                TerminationReason::Timeout => Err(AgentError::Timeout),
                TerminationReason::Cancelled => {
                    Err(AgentError::InvalidResponse("Cancelled".into()))
                }
                _ => Err(AgentError::InvalidResponse("Unknown termination".into())),
            },
            _ => Err(AgentError::InvalidResponse(
                "Unexpected terminal state".into(),
            )),
        }
    }
}
