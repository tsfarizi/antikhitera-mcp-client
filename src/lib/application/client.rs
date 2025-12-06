use super::tooling::{ServerManager, ToolServerInterface};
use crate::config::{AppConfig, ModelProviderConfig, ServerConfig, ToolConfig};
use crate::model::{ModelError, ModelProvider, ModelRequest};
use crate::types::{ChatMessage, MessageRole};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

const LANGUAGE_GUIDANCE: &str = "Kamu memahami permintaan dalam bahasa apa pun. Jawablah menggunakan bahasa yang sama dengan warga kecuali mereka secara eksplisit meminta sebaliknya. Jangan gunakan tool apa pun hanya untuk menerjemahkan; tangani kebutuhan bahasa secara internal.";
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub default_provider: String,
    pub default_model: String,
    pub default_system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub prompt_template: Option<String>,
    pub providers: Vec<ModelProviderConfig>,
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
        }
    }
}

#[derive(Debug)]
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
        logs.push(format!("Provider '{provider}' dengan model '{model}'"));
        if !history.is_empty() {
            logs.push(format!(
                "Riwayat percakapan sebelumnya: {} pesan",
                history.len()
            ));
        }

        let mut messages = Vec::with_capacity(history.len() + 2);
        let system_prompt = self.compose_system_prompt(system);
        if !system_prompt.is_empty() {
            logs.push(format!(
                "System prompt aktif: {}",
                Self::summarise(&system_prompt)
            ));
            messages.push(ChatMessage::new(MessageRole::System, system_prompt));
        }
        let prompt_preview = Self::summarise(&request.prompt);
        messages.extend(history.iter().cloned());
        messages.push(ChatMessage::new(MessageRole::User, request.prompt.clone()));
        logs.push(format!("Pengguna: {prompt_preview}"));

        info!(
            session_id = session_id.as_str(),
            provider = provider.as_str(),
            model = model.as_str(),
            "Mengirim permintaan ke penyedia model"
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
            "Respons diterima dari penyedia model"
        );
        for entry in &logs {
            info!(session_id = final_session.as_str(), %entry, "Log interaksi");
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

        // If no template is set, just return the custom instruction
        if template.is_empty() {
            return custom_instruction.trim().to_string();
        }

        let tool_guidance = if self.config.tools.is_empty() {
            "Saat warga meminta layanan khusus di luar kemampuanmu saat ini, sampaikan permintaan maaf secara sopan dan jelaskan bahwa layanan tersebut belum tersedia. Tetap berikan alternatif manual atau informasi lain yang dapat membantu."
                .to_string()
        } else {
            let mut text = String::from(
                "Berikut tool layanan digital yang dapat kamu panggil bila diperlukan:\n",
            );
            for tool in &self.config.tools {
                let description = tool
                    .description
                    .as_deref()
                    .unwrap_or("Tidak ada deskripsi.");
                text.push_str(&format!("- {}: {}\n", tool.name, description));
            }
            text.push_str("Gunakan tool hanya saat benar-benar membantu warga. Jika permintaan tidak tercakup oleh tool yang tersedia, sampaikan permintaan maaf dan jelaskan keterbatasan yang ada.");
            text
        };

        let mut prompt = template
            .replace("{{language_guidance}}", LANGUAGE_GUIDANCE)
            .replace("{{tool_guidance}}", tool_guidance.trim())
            .replace("{{custom_instruction}}", custom_instruction.trim());

        // Clean leftover placeholders if template omits them.
        prompt = prompt
            .replace("{{language_guidance}}", "")
            .replace("{{tool_guidance}}", "")
            .replace("{{custom_instruction}}", "");

        // Normalise whitespace while preserving intentional blank lines.
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
            return "(kosong)".to_string();
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
    use crate::model::ModelResponse;
    use async_trait::async_trait;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct RecordingProvider {
        records: Arc<Mutex<Vec<ModelRequest>>>,
    }

    #[async_trait]
    impl ModelProvider for RecordingProvider {
        async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
            let mut lock = self.records.lock().await;
            lock.push(request.clone());
            Ok(ModelResponse {
                message: ChatMessage::new(MessageRole::Assistant, "ack"),
                session_id: request.session_id.clone(),
            })
        }
    }

    impl RecordingProvider {
        async fn records(&self) -> Vec<ModelRequest> {
            self.records.lock().await.clone()
        }
    }

    #[tokio::test]
    async fn generates_session_and_persists_history() {
        let provider = RecordingProvider::default();
        let client = McpClient::new(
            provider.clone(),
            ClientConfig::new("ollama", "llama3").with_system_prompt("be precise"),
        );

        let first = client
            .chat(ChatRequest {
                prompt: "hello".into(),
                provider: None,
                model: None,
                system_prompt: None,
                session_id: None,
            })
            .await
            .expect("first call succeeds");

        let second = client
            .chat(ChatRequest {
                prompt: "next".into(),
                provider: None,
                model: None,
                system_prompt: None,
                session_id: Some(first.session_id.clone()),
            })
            .await
            .expect("second call succeeds");

        assert_eq!(first.session_id, second.session_id);
        assert_eq!(first.provider, "ollama");
        assert_eq!(second.provider, "ollama");
        assert_eq!(first.model, "llama3");
        assert_eq!(second.model, "llama3");
        assert!(!first.logs.is_empty());
        assert!(!second.logs.is_empty());

        let records = provider.records().await;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].provider, "ollama");
        assert_eq!(records[1].provider, "ollama");

        let first_messages = &records[0].messages;
        assert_eq!(first_messages.len(), 2);
        assert_eq!(first_messages[0].role, MessageRole::System);

        let second_messages = &records[1].messages;
        assert_eq!(second_messages.len(), 4);
        assert_eq!(second_messages[1].role, MessageRole::User);
        assert_eq!(second_messages[2].role, MessageRole::Assistant);
    }
}
