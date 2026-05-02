//! CLI domain entities.
//!
//! Generic message/action types are re-exported from `antikythera-core`.
//! Provider-routing types (`ProviderType`, `ProviderConfig`, `ChatSession`)
//! are owned here because they represent CLI-level wiring concerns, not
//! core protocol semantics.

pub use antikythera_core::domain::entities::{
    AgentAction, Message, MessageRole, ToolCall, ToolResult,
};

use chrono;
use serde::{Deserialize, Serialize};

/// Identifies the HTTP client implementation used for a provider.
///
/// This is a CLI routing tag — core has no knowledge of these names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Gemini,
    Ollama,
    OpenAi,
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
            "openai" => Ok(ProviderType::OpenAi),
            _ => Err(format!("Unknown provider type: {value}")),
        }
    }
}

/// Resolved provider connection config used by CLI infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: ProviderType,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
}

/// CLI-level chat session state.
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
