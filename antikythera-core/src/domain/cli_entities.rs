use serde::{Deserialize, Serialize};

/// Chat message in conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// LLM provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Gemini,
    Ollama,
}

impl ProviderType {
    pub fn parse(value: &str) -> Option<Self> {
        value.parse::<Self>().ok()
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "gemini" => Ok(ProviderType::Gemini),
            "ollama" => Ok(ProviderType::Ollama),
            _ => Err(format!("Unknown provider type: {value}")),
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: ProviderType,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
}

/// Tool call from LLM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool execution result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

/// Agent action determined from model response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentAction {
    CallTool(ToolCall),
    FinalResponse(String),
    Error(String),
}

/// Chat session state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatSession {
    pub id: String,
    pub messages: Vec<Message>,
    pub provider: ProviderConfig,
    pub agent_mode: bool,
    pub max_steps: u32,
    pub current_step: u32,
}

impl ChatSession {
    pub fn new(provider: ProviderConfig) -> Self {
        Self {
            id: format!("session-{}", chrono::Utc::now().timestamp_millis()),
            messages: Vec::new(),
            provider,
            agent_mode: true,
            max_steps: 10,
            current_step: 0,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn is_max_steps_exceeded(&self) -> bool {
        self.current_step >= self.max_steps
    }

    pub fn reset(&mut self) {
        self.messages.clear();
        self.current_step = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type_parse_is_case_insensitive() {
        assert_eq!(ProviderType::parse("GeMiNi"), Some(ProviderType::Gemini));
        assert_eq!(ProviderType::parse("OLLAMA"), Some(ProviderType::Ollama));
        assert_eq!(ProviderType::parse("unknown"), None);
    }

    #[test]
    fn chat_session_starts_with_defaults() {
        let provider = ProviderConfig {
            id: "p1".to_string(),
            provider_type: ProviderType::Ollama,
            endpoint: "http://127.0.0.1:11434".to_string(),
            api_key: None,
            model: "llama3".to_string(),
        };

        let session = ChatSession::new(provider);
        assert!(session.id.starts_with("session-"));
        assert!(session.messages.is_empty());
        assert!(session.agent_mode);
        assert_eq!(session.max_steps, 10);
        assert_eq!(session.current_step, 0);
    }

    #[test]
    fn chat_session_max_steps_works() {
        let provider = ProviderConfig {
            id: "p1".to_string(),
            provider_type: ProviderType::Gemini,
            endpoint: "https://example.com".to_string(),
            api_key: Some("ENV_KEY".to_string()),
            model: "gemini-pro".to_string(),
        };

        let mut session = ChatSession::new(provider);
        session.current_step = 10;
        assert!(session.is_max_steps_exceeded());
    }
}
