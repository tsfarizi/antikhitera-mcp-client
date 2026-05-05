use crate::application::agent::{Agent, AgentOptions, AgentStep};
use crate::application::client::{ChatRequest, McpClient};
use crate::application::model_provider::ModelProvider;
use crate::domain::types::MessagePart;
use crate::logging::ChatLogger;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct ChatService<P: ModelProvider> {
    client: Arc<McpClient<P>>,
}

pub struct ChatServiceOutcome {
    pub logs: Option<Vec<String>>,
    pub session_id: String,
    pub content: Value,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tool_steps: Option<Vec<AgentStep>>,
}

impl<P: ModelProvider> ChatService<P> {
    pub fn new(client: Arc<McpClient<P>>) -> Self {
        Self { client }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn process_request(
        &self,
        prompt: String,
        attachments: Vec<MessagePart>,
        system_prompt: Option<String>,
        session_id: Option<String>,
        agent_enabled: bool,
        max_tool_steps: Option<usize>,
        debug_mode: bool,
    ) -> Result<ChatServiceOutcome, String> {
        if agent_enabled {
            let provider = self.client.default_provider().to_string();
            let model = self.client.default_model().to_string();

            self.run_agent(
                prompt,
                attachments,
                system_prompt,
                session_id,
                max_tool_steps,
                debug_mode,
                provider,
                model,
            )
            .await
        } else {
            self.run_raw_chat(prompt, attachments, system_prompt, session_id, debug_mode)
                .await
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_agent(
        &self,
        prompt: String,
        attachments: Vec<MessagePart>,
        system_prompt: Option<String>,
        session_id: Option<String>,
        max_tool_steps: Option<usize>,
        debug_mode: bool,
        provider: String,
        model: String,
    ) -> Result<ChatServiceOutcome, String> {
        let log_session = session_id.clone();
        let mut options = AgentOptions {
            system_prompt,
            session_id,
            attachments,
            ..AgentOptions::default()
        };
        if let Some(max_steps) = max_tool_steps {
            options.max_steps = max_steps;
        }

        let agent_runner = Agent::new(self.client.clone());
        match agent_runner.run_ui_layout(prompt, options).await {
            Ok((outcome, content_json)) => {
                ChatLogger::new(&outcome.session_id).info("Agent run completed successfully");

                // Sync agent steps to session manager
                self.client
                    .record_agent_outcome(&outcome.session_id, &outcome.steps)
                    .await;

                Ok(self.construct_outcome(
                    debug_mode,
                    outcome.session_id,
                    content_json,
                    outcome.logs,
                    provider,
                    model,
                    outcome.steps,
                ))
            }
            Err(error) => {
                ChatLogger::new(log_session.as_deref().unwrap_or("tui"))
                    .error(format!("Agent run failed | error={}", error));
                Err(error.user_message())
            }
        }
    }

    async fn run_raw_chat(
        &self,
        prompt: String,
        attachments: Vec<MessagePart>,
        system_prompt: Option<String>,
        session_id: Option<String>,
        debug_mode: bool,
    ) -> Result<ChatServiceOutcome, String> {
        let log = ChatLogger::new(session_id.as_deref().unwrap_or("tui"));
        log.debug("Forwarding /chat request to model provider (raw mode)");
        let result = self
            .client
            .chat(ChatRequest {
                prompt,
                attachments,
                system_prompt,
                session_id,
                raw_mode: true,
                bypass_template: false, // Not relevant when raw_mode is true
                force_json: false,
            })
            .await;

        match result {
            Ok(result) => {
                log.info(format!(
                    "Chat request completed successfully | session_id={} provider={} model={}",
                    result.session_id.as_str(),
                    result.provider.as_str(),
                    result.model.as_str()
                ));

                let content = json!(result.content);

                Ok(self.construct_outcome(
                    debug_mode,
                    result.session_id,
                    content,
                    result.logs,
                    result.provider,
                    result.model,
                    Vec::new(),
                ))
            }
            Err(error) => {
                log.error(format!(
                    "Model provider returned an error | error={}",
                    error
                ));
                Err(error.to_string())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn construct_outcome(
        &self,
        debug: bool,
        session_id: String,
        content: Value,
        logs: Vec<String>,
        provider: String,
        model: String,
        tool_steps: Vec<AgentStep>,
    ) -> ChatServiceOutcome {
        if !debug {
            ChatServiceOutcome {
                logs: None,
                session_id,
                content,
                provider: None,
                model: None,
                tool_steps: None,
            }
        } else {
            ChatServiceOutcome {
                logs: Some(logs),
                session_id,
                content,
                provider: Some(provider),
                model: Some(model),
                tool_steps: Some(tool_steps),
            }
        }
    }
}

