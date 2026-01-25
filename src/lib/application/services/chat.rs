use crate::agent::{Agent, AgentOptions, AgentStep};
use crate::application::client::{ChatRequest, McpClient, McpError};
use crate::model::ModelProvider;
use crate::types::MessagePart;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};

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
            self.run_raw_chat(
                prompt,
                attachments,
                system_prompt,
                session_id,
                debug_mode,
            )
            .await
        }
    }

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
        let mut options = AgentOptions::default();
        options.system_prompt = system_prompt;
        options.session_id = session_id;
        options.attachments = attachments;
        if let Some(max_steps) = max_tool_steps {
            options.max_steps = max_steps;
        }

        let agent_runner = Agent::new(self.client.clone());
        match agent_runner.run_ui_layout(prompt, options).await {
            Ok((outcome, content_json)) => {
                info!(
                    session_id = outcome.session_id.as_str(),
                    "Agent run completed successfully"
                );

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
                error!(%error, "Agent run failed");
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
        debug!("Forwarding /chat request to model provider (raw mode)");
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
                info!(
                    session_id = result.session_id.as_str(),
                    provider = result.provider.as_str(),
                    model = result.model.as_str(),
                    "Chat request completed successfully"
                );

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
            Err(McpError::Model(error)) => {
                error!(%error, "Model provider returned an error");
                Err(error.user_message())
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::client::ClientConfig;
    use crate::infrastructure::model::types::{ModelError, ModelRequest, ModelResponse};
    use crate::types::ChatMessage;
    use async_trait::async_trait;

    struct MockModelProvider {
        response_content: String,
    }

    #[async_trait]
    impl ModelProvider for MockModelProvider {
        async fn chat(&self, _request: ModelRequest) -> Result<ModelResponse, ModelError> {
            Ok(ModelResponse {
                message: ChatMessage::new(crate::types::MessageRole::Assistant, self.response_content.clone()),
                session_id: None,
            })
        }
    }

    #[tokio::test]
    async fn test_chat_service_raw_mode_no_debug() {
        let provider = MockModelProvider {
            response_content: "Hello world".to_string(),
        };
        let config = ClientConfig::new("test-provider", "test-model");
        let client = Arc::new(McpClient::new(provider, config));
        let service = ChatService::new(client);

        let result = service
            .process_request(
                "hi".to_string(),
                vec![],
                None,
                None,
                false, // agent disabled
                None,
                false, // debug disabled
            )
            .await;

        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert_eq!(outcome.content, json!("Hello world"));
        assert!(outcome.logs.is_none());
        assert!(outcome.provider.is_none());
    }

    #[tokio::test]
    async fn test_chat_service_raw_mode_with_debug() {
        let provider = MockModelProvider {
            response_content: "Hello world".to_string(),
        };
        let config = ClientConfig::new("test-provider", "test-model");
        let client = Arc::new(McpClient::new(provider, config));
        let service = ChatService::new(client);

        let result = service
            .process_request(
                "hi".to_string(),
                vec![],
                None,
                None,
                false, // agent disabled
                None,
                true, // debug enabled
            )
            .await;

        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert_eq!(outcome.content, json!("Hello world"));
        assert!(outcome.logs.is_some());
        assert!(outcome.provider.is_some());
        assert_eq!(outcome.provider.unwrap(), "test-provider");
    }
}
