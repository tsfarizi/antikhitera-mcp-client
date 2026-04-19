//! # MCP Client Module
//!
//! This module provides the core MCP client implementation for communicating
//! with AI language models. It handles chat sessions, tool execution, and
//! conversation management.
//!
//! ## Key Types
//!
//! - [`McpClient`] - Main client for model communication
//! - [`ClientConfig`] - Configuration for the client
//! - [`ChatRequest`] - Request parameters for a chat
//! - [`ChatResult`] - Response from a chat request
//!
//! ## Example
//!
//! ```no_run,ignore
//! use antikythera_core::client::{McpClient, ClientConfig, ChatRequest};
//!
//! async fn example() {
//!     // Client setup would go here
//! }
//! ```

use super::tooling::{ServerManager, ToolServerInterface};
use crate::config::{AppConfig, ModelProviderConfig, PromptsConfig, ServerConfig, ToolConfig};
use crate::infrastructure::model::{HostModelResponse, ModelError, ModelProvider, ModelRequest, ModelResponse};
use crate::domain::types::MessagePart;
use crate::domain::types::{ChatMessage, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

/// Client configuration for the MCP client.
///
/// This struct holds all settings needed to initialize and run the client,
/// including provider settings, tools, servers, and prompt configurations.
///
/// Use the builder pattern methods (`with_*`) to construct the configuration:
///
/// ```no_run,ignore
/// use antikythera_core::client::ClientConfig;
///
/// let config = ClientConfig::new("gemini", "gemini-2.0-flash")
///     .with_system_prompt("You are a helpful assistant.");
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Default provider ID to use
    pub default_provider: String,
    /// Default model name
    pub default_model: String,
    /// Optional system prompt override
    pub default_system_prompt: Option<String>,
    /// Available tools from MCP servers
    pub tools: Vec<ToolConfig>,
    /// MCP server configurations
    pub servers: Vec<ServerConfig>,
    /// Available model providers
    pub providers: Vec<ModelProviderConfig>,
    /// Configurable prompts for agent behavior
    pub prompts: PromptsConfig,
}

impl ClientConfig {
    /// Create a new client configuration with the specified provider and model.
    pub fn new(default_provider: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            default_provider: default_provider.into(),
            default_model: default_model.into(),
            default_system_prompt: None,
            tools: Vec::new(),
            servers: Vec::new(),
            providers: Vec::new(),
            prompts: PromptsConfig::default(),
        }
    }

    /// Set the default system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.default_system_prompt = Some(prompt.into());
        self
    }

    /// Set the available tools.
    pub fn with_tools(mut self, tools: Vec<ToolConfig>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the MCP server configurations.
    pub fn with_servers(mut self, servers: Vec<ServerConfig>) -> Self {
        self.servers = servers;
        self
    }

    /// Set the available model providers.
    pub fn with_providers(mut self, providers: Vec<ModelProviderConfig>) -> Self {
        self.providers = providers;
        self
    }

    /// Set the prompts configuration.
    pub fn with_prompts(mut self, prompts: PromptsConfig) -> Self {
        self.prompts = prompts;
        self
    }

    /// Get the list of providers.
    pub fn providers(&self) -> &[ModelProviderConfig] {
        &self.providers
    }

    /// Get the prompt template from prompts config.
    pub fn prompt_template(&self) -> &str {
        self.prompts.template()
    }

    /// Convert to AppConfig for compatibility.
    pub fn to_app_config(&self) -> AppConfig {
        AppConfig {
            default_provider: self.default_provider.clone(),
            model: self.default_model.clone(),
            system_prompt: self.default_system_prompt.clone(),
            tools: self.tools.clone(),
            servers: self.servers.clone(),
            providers: self.providers.clone(),
            rest_server: Default::default(),
            prompts: self.prompts.clone(),
        }
    }
}

/// Request parameters for a chat interaction.
#[derive(Debug, Default)]
pub struct ChatRequest {
    /// The user's message/prompt
    pub prompt: String,
    /// Optional file/image attachments
    pub attachments: Vec<MessagePart>,
    /// Optional system prompt override
    pub system_prompt: Option<String>,
    /// Session ID for conversation continuity
    pub session_id: Option<String>,
    /// Raw mode - bypass all config system prompts and templates
    /// Used for direct model queries without context injection
    pub raw_mode: bool,
    /// Skip template composition - use system_prompt as-is
    /// Used by Agent runner which composes its own complete system prompt
    pub bypass_template: bool,
    /// Force JSON mode - requests the LLM to output valid JSON
    pub force_json: bool,
}

/// Result from a chat interaction.
///
/// Contains the model's response along with metadata about
/// the interaction.
#[derive(Debug, Clone)]
pub struct ChatResult {
    /// The model's response content
    pub content: String,
    /// Session ID for this conversation
    pub session_id: String,
    /// Provider used for this request
    pub provider: String,
    /// Model used for this request
    pub model: String,
    /// Debug/execution logs
    pub logs: Vec<String>,
}

/// Prepared host-facing model request.
///
/// The host owns the actual LLM API call. This struct captures the exact
/// request payload plus the session metadata needed to commit the response
/// back into the client's internal history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedChatTurn {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub model_request: ModelRequest,
    pub user_message: ChatMessage,
    pub logs: Vec<String>,
}

#[derive(Debug, Error)]
pub enum McpError {
    #[error(transparent)]
    Model(#[from] ModelError),
}

impl McpError {
    pub fn user_message(&self) -> String {
        match self {
            McpError::Model(err) => err.user_message(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfigSnapshot {
    pub model: String,
    pub default_provider: String,
    pub system_prompt: Option<String>,
    pub prompt_template: String,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub providers: Vec<ModelProviderConfig>,
    pub raw: String,
}

pub struct McpClient<P: ModelProvider> {
    provider: P,
    config: ClientConfig,
    sessions: Mutex<HashMap<String, Vec<ChatMessage>>>,
    server_bridge: Arc<dyn ToolServerInterface>,
}

impl<P: ModelProvider> McpClient<P> {
    pub fn new(provider: P, config: ClientConfig) -> Self {
        let server_manager = Arc::new(ServerManager::new(config.servers.clone()));
        let bridge: Arc<dyn ToolServerInterface> = server_manager;
        Self {
            provider,
            config,
            sessions: Mutex::new(HashMap::new()),
            server_bridge: bridge,
        }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub fn tools(&self) -> &[ToolConfig] {
        &self.config.tools
    }

    pub fn default_provider(&self) -> &str {
        &self.config.default_provider
    }

    pub fn default_model(&self) -> &str {
        &self.config.default_model
    }

    pub fn config_snapshot(&self) -> ClientConfigSnapshot {
        let app_config = self.config.to_app_config();
        let prompt_template = app_config.prompt_template().to_string();
        let raw = app_config.to_raw_toml();
        ClientConfigSnapshot {
            model: app_config.model.clone(),
            default_provider: app_config.default_provider.clone(),
            system_prompt: app_config.system_prompt.clone(),
            prompt_template,
            tools: app_config.tools.clone(),
            servers: app_config.servers.clone(),
            providers: app_config.providers.clone(),
            raw,
        }
    }

    pub fn providers(&self) -> &[ModelProviderConfig] {
        self.config.providers()
    }

    pub fn prompts(&self) -> &PromptsConfig {
        &self.config.prompts
    }

    pub fn server_bridge(&self) -> Arc<dyn ToolServerInterface> {
        self.server_bridge.clone()
    }

    pub async fn prepare_chat(&self, request: ChatRequest) -> PreparedChatTurn {
        let provider = self.config.default_provider.clone();
        let model = self.config.default_model.clone();
        let session_id = request.session_id.clone().unwrap_or_else(new_session_id);
        let raw_mode = request.raw_mode;

        let mut logs = Vec::new();
        logs.push(format!("Provider '{provider}' with model '{model}'"));

        let mut messages = Vec::new();

        if raw_mode {
            // Raw mode: skip all system prompts and history, just send user message directly
            logs.push("Raw mode: sending request directly to model".to_string());
        } else {
            // Normal mode: include system prompt and history
            let history = {
                let start_wait = std::time::Instant::now();
                let mut sessions = self.sessions.lock().await;
                let elapsed = start_wait.elapsed();
                tracing::debug!(lock_wait_us = ?elapsed.as_micros(), "Acquired session lock for reading history");
                sessions.entry(session_id.clone()).or_default().clone()
            };
            debug!(
                session_id = session_id.as_str(),
                history_count = history.len(),
                "Preparing chat request with prior history"
            );

            if !history.is_empty() {
                logs.push(format!(
                    "Previous conversation history: {} messages",
                    history.len()
                ));
            }

            // Determine system prompt based on bypass_template flag
            let system_prompt = if request.bypass_template {
                // Agent provides complete system prompt - use as-is
                request.system_prompt.unwrap_or_default()
            } else {
                // Normal mode: compose with template
                let system = request
                    .system_prompt
                    .or_else(|| self.config.default_system_prompt.clone());
                self.compose_system_prompt(system)
            };

            if !system_prompt.is_empty() {
                logs.push(format!(
                    "System prompt active: {}",
                    Self::summarise(&system_prompt)
                ));
                messages.push(ChatMessage::new(MessageRole::System, system_prompt));
            }
            messages.extend(history.iter().cloned());
        }

        let mut user_parts = vec![MessagePart::text(request.prompt.clone())];
        user_parts.extend(request.attachments.clone());
        let user_message = ChatMessage::with_parts(MessageRole::User, user_parts);
        let prompt_preview = Self::summarise(&request.prompt);
        messages.push(user_message.clone());

        if !request.attachments.is_empty() {
            logs.push(format!(
                "User: {} (with {} attachment(s))",
                prompt_preview,
                request.attachments.len()
            ));
        } else {
            logs.push(format!("User: {prompt_preview}"));
        }

        PreparedChatTurn {
            session_id: session_id.clone(),
            provider: provider.clone(),
            model: model.clone(),
            model_request: ModelRequest {
                provider: provider.clone(),
                model: model.clone(),
                messages,
                session_id: Some(session_id.clone()),
                force_json: request.force_json,
            },
            user_message: user_message.clone(),
            logs,
        }
    }

    pub async fn complete_chat(
        &self,
        prepared: PreparedChatTurn,
        response: ModelResponse,
    ) -> Result<ChatResult, McpError> {
        let final_session = response
            .session_id
            .clone()
            .unwrap_or_else(|| prepared.session_id.clone());
        let assistant_message = response.message.clone();
        let response_preview = Self::summarise(&assistant_message.content());

        let mut logs = prepared.logs;
        logs.push(format!("Model: {response_preview}"));

        info!(
            session_id = final_session.as_str(),
            provider = prepared.provider.as_str(),
            model = prepared.model.as_str(),
            "Response received from model provider"
        );
        for entry in &logs {
            info!(session_id = final_session.as_str(), %entry, "Interaction log");
        }

        self.persist_exchange(&final_session, prepared.user_message, assistant_message)
            .await;

        Ok(ChatResult {
            content: response.message.content(),
            session_id: final_session,
            provider: prepared.provider,
            model: prepared.model,
            logs,
        })
    }

    pub async fn complete_chat_from_host(
        &self,
        prepared: PreparedChatTurn,
        response: HostModelResponse,
    ) -> Result<ChatResult, McpError> {
        let provider = prepared.provider.clone();
        let model_response = response.into_model_response(&provider)?;
        self.complete_chat(prepared, model_response).await
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResult, McpError> {
        let prepared = self.prepare_chat(request).await;

        info!(
            session_id = prepared.session_id.as_str(),
            provider = prepared.provider.as_str(),
            model = prepared.model.as_str(),
            "Dispatching prepared request to model host"
        );

        let response = self.provider.chat(prepared.model_request.clone()).await?;
        self.complete_chat(prepared, response).await
    }

    fn compose_system_prompt(&self, override_prompt: Option<String>) -> String {
        let template = self.config.prompt_template().to_string();
        let custom_instruction = override_prompt.unwrap_or_default();
        if template.is_empty() {
            return custom_instruction.trim().to_string();
        }

        let tool_guidance = if self.config.tools.is_empty() {
            // No tools available - use fallback guidance
            self.config.prompts.fallback_guidance().to_string()
        } else {
            // Tools available - list them with guidance
            let mut text = format!("{}\n", self.config.prompts.tool_guidance());
            for tool in &self.config.tools {
                let description = tool
                    .description
                    .as_deref()
                    .unwrap_or("No description available.");
                text.push_str(&format!("- {}: {}\n", tool.name, description));
            }
            text.push_str(self.config.prompts.fallback_guidance());
            text
        };

        let mut prompt = template
            .replace("{{language_guidance}}", "")
            .replace("{{tool_guidance}}", tool_guidance.trim())
            .replace("{{custom_instruction}}", custom_instruction.trim());
        prompt = prompt
            .replace("{{language_guidance}}", "")
            .replace("{{tool_guidance}}", "")
            .replace("{{custom_instruction}}", "");
        let mut cleaned = Vec::new();
        let mut previous_blank = false;
        for line in prompt.lines().map(|line| line.trim_end()) {
            let trimmed = line.trim();
            let is_blank = trimmed.is_empty();
            if is_blank {
                if !previous_blank {
                    cleaned.push(String::new());
                }
                previous_blank = true;
            } else {
                cleaned.push(trimmed.to_string());
                previous_blank = false;
            }
        }

        cleaned.join("\n").trim().to_string()
    }

    async fn persist_exchange(
        &self,
        session_id: &str,
        user_message: ChatMessage,
        assistant: ChatMessage,
    ) {
        let start_wait = std::time::Instant::now();
        let mut sessions = self.sessions.lock().await;
        let elapsed = start_wait.elapsed();
        tracing::debug!(lock_wait_us = ?elapsed.as_micros(), "Acquired session lock to persist exchange");

        let history = sessions.entry(session_id.to_string()).or_default();
        history.push(user_message);
        history.push(assistant);
        debug!(
            session_id,
            total_messages = history.len(),
            "Persisted chat exchange to session history"
        );
    }

    pub(crate) fn summarise(text: &str) -> String {
        const SNIPPET_LIMIT: usize = 160;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return "(empty)".to_string();
        }
        let single_line = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut result = String::new();
        let mut chars = single_line.chars();
        for _ in 0..SNIPPET_LIMIT {
            if let Some(ch) = chars.next() {
                result.push(ch);
            } else {
                return result;
            }
        }
        if chars.next().is_some() {
            result.push('…');
        }
        result
    }
}

fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockProvider {
        response: String,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl ModelProvider for MockProvider {
        async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
            Ok(ModelResponse::new(
                format!("{}:{}", request.session_id.unwrap_or_default(), self.response),
                None,
            ))
        }
    }

    #[tokio::test]
    async fn prepare_and_complete_chat_preserve_session_history() {
        let client = McpClient::new(
            MockProvider {
                response: "siap".to_string(),
            },
            ClientConfig::new("host", "gpt-host"),
        );

        let first = client
            .chat(ChatRequest {
                prompt: "halo".to_string(),
                attachments: Vec::new(),
                system_prompt: None,
                session_id: None,
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await
            .unwrap();

        let prepared = client
            .prepare_chat(ChatRequest {
                prompt: "lanjut".to_string(),
                attachments: Vec::new(),
                system_prompt: None,
                session_id: Some(first.session_id.clone()),
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await;

        assert_eq!(prepared.session_id, first.session_id);
            assert!(prepared.model_request.messages.len() >= 3);
            assert!(prepared
                .model_request
                .messages
                .iter()
                .any(|message| message.content() == "halo"));
            assert!(prepared
                .model_request
                .messages
                .iter()
                .any(|message| message.content().contains("siap")));
            assert_eq!(
                prepared.model_request.messages.last().unwrap().content(),
                "lanjut"
            );
    }

    #[tokio::test]
    async fn complete_chat_from_host_accepts_plain_text_and_preserves_attachments() {
        let client = McpClient::new(
            MockProvider {
                response: "unused".to_string(),
            },
            ClientConfig::new("host", "gpt-host"),
        );

        let prepared = client
            .prepare_chat(ChatRequest {
                prompt: "lihat lampiran".to_string(),
                attachments: vec![MessagePart::file("a.txt", "text/plain", "ZGF0YQ==")],
                system_prompt: None,
                session_id: Some("sess-attach".to_string()),
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await;

        let result = client
            .complete_chat_from_host(
                prepared,
                HostModelResponse::from_text("berhasil", Some("sess-attach".to_string())),
            )
            .await
            .unwrap();

        let follow_up = client
            .prepare_chat(ChatRequest {
                prompt: "cek riwayat".to_string(),
                attachments: Vec::new(),
                system_prompt: None,
                session_id: Some(result.session_id.clone()),
                raw_mode: false,
                bypass_template: false,
                force_json: false,
            })
            .await;

            assert!(follow_up
                .model_request
                .messages
                .iter()
                .any(|message| message.has_attachments() && message.content() == "lihat lampiran"));
            assert!(follow_up
                .model_request
                .messages
                .iter()
                .any(|message| message.content() == "berhasil"));
            assert_eq!(
                follow_up.model_request.messages.last().unwrap().content(),
                "cek riwayat"
            );
    }
}
