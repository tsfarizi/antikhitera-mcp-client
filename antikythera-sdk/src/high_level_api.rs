//! High-level API for MCP client operations.

use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome};
use antikythera_core::application::client::{
    ChatRequest, ChatResult, ClientConfig, McpClient, PreparedChatTurn,
};
use antikythera_core::config::{AppConfig, ToolConfig};
use antikythera_core::infrastructure::model::{
    DynamicModelProvider, HostModelClient, HostModelResponse, HostModelTransport,
};
use std::sync::Arc;
use thiserror::Error;

/// Lightweight provider descriptor used by the SDK to register host transport backends.
///
/// Unlike the CLI-owned `ModelProviderConfig`, this struct only carries the
/// fields the SDK core needs for routing: an ID and an optional list of model
/// names that the provider accepts.
#[derive(Debug, Clone, Default)]
pub struct ProviderEntry {
    /// Provider ID (must match the `default_provider` field in `AppConfig`).
    pub id: String,
    /// Accepted model names. An empty list means "accept any model".
    pub models: Vec<String>,
}

/// High-level MCP client wrapper.
pub struct Client {
    core_client: Arc<McpClient<DynamicModelProvider>>,
    direct_model_dispatch: bool,
}

impl Client {
    /// Create a new client from configuration.
    ///
    /// This constructor does not create any model HTTP client. Use
    /// [`with_host_transport`](Self::with_host_transport) if the host wants the
    /// SDK to delegate model calls automatically through a host transport.
    pub async fn new(config: AppConfig) -> Result<Self, SdkError> {
        let client_config = Self::build_client_config(&config);

        let core_client = Arc::new(McpClient::new(DynamicModelProvider::new(), client_config));
        Ok(Self {
            core_client,
            direct_model_dispatch: false,
        })
    }

    /// Create a new client that delegates all model calls to a host transport.
    ///
    /// `providers` specifies the provider IDs and their accepted model lists so
    /// the routing layer knows which backends are available.  Pass an empty
    /// `Vec` to register a single catch-all provider for the `default_provider`
    /// in `config`.
    pub async fn with_host_transport(
        config: AppConfig,
        providers: Vec<ProviderEntry>,
        transport: Arc<dyn HostModelTransport>,
    ) -> Result<Self, SdkError> {
        let client_config = Self::build_client_config(&config);
        let mut provider = DynamicModelProvider::new();

        let entries = if providers.is_empty() {
            // Fallback: register the default provider with an empty model allow-list
            // (accepts any model name).
            vec![ProviderEntry {
                id: config.default_provider.clone(),
                models: Vec::new(),
            }]
        } else {
            providers
        };

        for entry in &entries {
            provider = provider.register(
                entry.id.clone(),
                entry.models.clone(),
                Box::new(HostModelClient::new(entry.id.clone(), transport.clone())),
            );
        }

        let core_client = Arc::new(McpClient::new(provider, client_config));
        Ok(Self {
            core_client,
            direct_model_dispatch: true,
        })
    }

    fn build_client_config(config: &AppConfig) -> ClientConfig {
        ClientConfig::new(config.default_provider.clone(), config.model.clone())
            .with_tools(config.tools.clone())
            .with_servers(config.servers.clone())
            .with_prompts(config.prompts.clone())
    }

    /// Send a chat message and get response.
    pub async fn chat(&self, prompt: String) -> Result<String, SdkError> {
        if !self.direct_model_dispatch {
            return Err(SdkError::Unsupported(
                "Direct model dispatch is disabled. Use prepare_chat/complete_chat or construct the client with with_host_transport().".to_string(),
            ));
        }

        let request = ChatRequest {
            prompt,
            attachments: Vec::new(),
            system_prompt: None,
            session_id: None,
            raw_mode: false,
            bypass_template: false,
            force_json: false,
        };

        let response = self
            .core_client
            .chat(request)
            .await
            .map_err(|e| SdkError::Chat(e.to_string()))?;

        Ok(response.content)
    }

    /// Build the exact model request that the host should execute.
    pub async fn prepare_chat(
        &self,
        prompt: String,
        session_id: Option<String>,
    ) -> Result<PreparedChatTurn, SdkError> {
        Ok(self
            .core_client
            .prepare_chat(ChatRequest {
                prompt,
                attachments: Vec::new(),
                system_prompt: None,
                session_id,
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await)
    }

    /// Commit a host-produced model response back into session/history state.
    pub async fn complete_chat(
        &self,
        prepared: PreparedChatTurn,
        response: HostModelResponse,
    ) -> Result<ChatResult, SdkError> {
        self.core_client
            .complete_chat_from_host(prepared, response)
            .await
            .map_err(|e| SdkError::Chat(e.to_string()))
    }

    /// Run agent with autonomous tool execution.
    pub async fn run_agent(
        &self,
        prompt: String,
        options: AgentOptions,
    ) -> Result<AgentOutcome, SdkError> {
        if !self.direct_model_dispatch {
            return Err(SdkError::Unsupported(
                "Autonomous agent execution requires a host transport. Use with_host_transport() so model calls can be delegated to the host.".to_string(),
            ));
        }

        let agent = Agent::new(self.core_client.clone());
        agent
            .run(prompt, options)
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

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Chat error: {0}")]
    Chat(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
