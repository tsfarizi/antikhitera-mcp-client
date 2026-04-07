//! Domain entities

/// Chat message in conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: MessageRole::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: MessageRole::Assistant, content: content.into() }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self { role: MessageRole::System, content: content.into() }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self { role: MessageRole::Tool, content: content.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// LLM provider type
#[derive(Debug, Clone)]
pub enum ProviderType {
    Gemini,
    Ollama,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gemini" => Some(ProviderType::Gemini),
            "ollama" => Some(ProviderType::Ollama),
            _ => None,
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: ProviderType,
    pub endpoint: String,
    pub api_key: Option<String>, // None for Ollama
    pub model: String,
}

/// Tool call from LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

/// Agent action determined from LLM response
#[derive(Debug, Clone)]
pub enum AgentAction {
    CallTool(ToolCall),
    FinalResponse(String),
    Error(String),
}

/// Chat session state
#[derive(Debug, Clone)]
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
