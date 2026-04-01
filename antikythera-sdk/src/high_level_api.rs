//! High-level API for MCP client operations.

use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome};
use antikythera_core::application::client::{ChatRequest, ClientConfig, McpClient};
use antikythera_core::config::{AppConfig, ToolConfig};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use std::sync::Arc;
use thiserror::Error;

/// High-level MCP client wrapper.
pub struct Client {
    core_client: Arc<McpClient<DynamicModelProvider>>,
}

impl Client {
    /// Create a new client from configuration.
    pub async fn new(config: AppConfig) -> Result<Self, SdkError> {
        let provider = DynamicModelProvider::from_configs(&config.providers)
            .map_err(|e| SdkError::Configuration(e.to_string()))?;

        let client_config = ClientConfig::new(
            config.default_provider.clone(),
            config.model.clone(),
        )
        .with_tools(config.tools.clone())
        .with_servers(config.servers.clone());

        let core_client = Arc::new(McpClient::new(provider, client_config));
        Ok(Self { core_client })
    }

    /// Send a chat message and get response.
    pub async fn chat(&self, prompt: String) -> Result<String, SdkError> {
        let request = ChatRequest {
            prompt,
            attachments: Vec::new(),
            system_prompt: None,
            session_id: None,
            raw_mode: false,
            bypass_template: false,
            force_json: false,
        };

        let response = self.core_client
            .chat(request)
            .await
            .map_err(|e| SdkError::Chat(e.to_string()))?;

        Ok(response.content)
    }

    /// Run agent with autonomous tool execution.
    pub async fn run_agent(
        &self,
        prompt: String,
        options: AgentOptions,
    ) -> Result<AgentOutcome, SdkError> {
        let agent = Agent::new(self.core_client.clone());
        agent.run(prompt, options)
            .await
            .map_err(|e| SdkError::Agent(e.to_string()))
    }

    /// List available tools.
    pub fn list_tools(&self) -> Vec<ToolConfig> {
        self.core_client.tools().to_vec()
    }
}

/// SDK error types.
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Chat error: {0}")]
    Chat(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
