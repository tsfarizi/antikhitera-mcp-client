use super::tooling::{ServerManager, ToolServerInterface};
use crate::config::{AppConfig, ModelProviderConfig, PromptsConfig, ServerConfig, ToolConfig};
use crate::model::{ModelError, ModelProvider, ModelRequest};
use crate::types::{ChatMessage, MessageRole};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub default_provider: String,
    pub default_model: String,
    pub default_system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub prompt_template: Option<String>,
    pub providers: Vec<ModelProviderConfig>,
    pub prompts: PromptsConfig,
}

impl ClientConfig {
    pub fn new(default_provider: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            default_provider: default_provider.into(),
            default_model: default_model.into(),
            default_system_prompt: None,
            tools: Vec::new(),
            servers: Vec::new(),
            prompt_template: None,
            providers: Vec::new(),
            prompts: PromptsConfig::default(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.default_system_prompt = Some(prompt.into());
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolConfig>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_servers(mut self, servers: Vec<ServerConfig>) -> Self {
        self.servers = servers;
        self
    }

    pub fn with_prompt_template(mut self, template: Option<String>) -> Self {
        self.prompt_template = template;
        self
    }

    pub fn with_providers(mut self, providers: Vec<ModelProviderConfig>) -> Self {
        self.providers = providers;
        self
    }

    pub fn providers(&self) -> &[ModelProviderConfig] {
        &self.providers
    }

    pub fn prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn to_app_config(&self) -> AppConfig {
        AppConfig {
            default_provider: self.default_provider.clone(),
            model: self.default_model.clone(),
            system_prompt: self.default_system_prompt.clone(),
            tools: self.tools.clone(),
            servers: self.servers.clone(),
            prompt_template: self.prompt_template.clone().unwrap_or_default(),
            providers: self.providers.clone(),
            rest_server: Default::default(),
            prompts: self.prompts.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ChatRequest {
    pub prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub content: String,
    pub session_id: String,
    pub provider: String,
    pub model: String,
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
        let prompt_template = app_config.prompt_template.clone();
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

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResult, McpError> {
        let provider = request
            .provider
            .unwrap_or_else(|| self.config.default_provider.clone());
        let model = request
            .model
            .unwrap_or_else(|| self.config.default_model.clone());
        let session_id = request.session_id.unwrap_or_else(new_session_id);
        let system = request
            .system_prompt
            .or_else(|| self.config.default_system_prompt.clone());

        let history = {
            let mut sessions = self.sessions.lock().await;
            sessions.entry(session_id.clone()).or_default().clone()
        };
        debug!(
            session_id = session_id.as_str(),
            history_count = history.len(),
            "Preparing chat request with prior history"
        );

        let mut logs = Vec::new();
        logs.push(format!("Provider '{provider}' with model '{model}'"));
        if !history.is_empty() {
            logs.push(format!(
                "Previous conversation history: {} messages",
                history.len()
            ));
        }

        let mut messages = Vec::with_capacity(history.len() + 2);
        let system_prompt = self.compose_system_prompt(system);
        if !system_prompt.is_empty() {
            logs.push(format!(
                "System prompt active: {}",
                Self::summarise(&system_prompt)
            ));
            messages.push(ChatMessage::new(MessageRole::System, system_prompt));
        }
        let prompt_preview = Self::summarise(&request.prompt);
        messages.extend(history.iter().cloned());
        messages.push(ChatMessage::new(MessageRole::User, request.prompt.clone()));
        logs.push(format!("User: {prompt_preview}"));

        info!(
            session_id = session_id.as_str(),
            provider = provider.as_str(),
            model = model.as_str(),
            "Sending request to model provider"
        );

        let response = self
            .provider
            .chat(ModelRequest {
                provider: provider.clone(),
                model: model.clone(),
                messages,
                session_id: Some(session_id.clone()),
            })
            .await?;

        let final_session = response
            .session_id
            .clone()
            .unwrap_or_else(|| session_id.clone());
        let assistant_message = response.message.clone();
        let response_preview = Self::summarise(&assistant_message.content);
        logs.push(format!("Model: {response_preview}"));

        info!(
            session_id = final_session.as_str(),
            provider = provider.as_str(),
            model = model.as_str(),
            "Response received from model provider"
        );
        for entry in &logs {
            info!(session_id = final_session.as_str(), %entry, "Interaction log");
        }

        self.persist_exchange(&final_session, request.prompt, assistant_message)
            .await;

        Ok(ChatResult {
            content: response.message.content,
            session_id: final_session,
            provider,
            model,
            logs,
        })
    }

    fn compose_system_prompt(&self, override_prompt: Option<String>) -> String {
        let template = self.config.prompt_template.clone().unwrap_or_default();
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
        user_prompt: String,
        assistant: ChatMessage,
    ) {
        let mut sessions = self.sessions.lock().await;
        let history = sessions.entry(session_id.to_string()).or_default();
        history.push(ChatMessage::new(MessageRole::User, user_prompt));
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
