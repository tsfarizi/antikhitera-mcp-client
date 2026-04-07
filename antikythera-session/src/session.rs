//! Session and Message Types

use serde::{Deserialize, Serialize};

// ============================================================================
/// Message Role
// ============================================================================

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// User input
    User,
    /// Assistant response
    Assistant,
    /// System message
    System,
    /// Tool call result
    ToolResult,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::ToolResult => "tool_result",
        }
    }
}

// ============================================================================
/// Message
// ============================================================================

/// Single message in chat history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Tool call info (if role is ToolResult)
    pub tool_name: Option<String>,
    /// Tool arguments (JSON encoded, if applicable)
    pub tool_args: Option<String>,
    /// Step number in agent flow
    pub step: Option<u32>,
    /// Additional metadata (JSON encoded)
    pub metadata: Option<String>,
}

impl Message {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_name: None,
            tool_args: None,
            step: None,
            metadata: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_name: None,
            tool_args: None,
            step: None,
            metadata: None,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_name: None,
            tool_args: None,
            step: None,
            metadata: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(
        tool_name: impl Into<String>,
        content: impl Into<String>,
        args: Option<serde_json::Value>,
        step: u32,
    ) -> Self {
        Self {
            role: MessageRole::ToolResult,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_name: Some(tool_name.into()),
            tool_args: args.map(|a| serde_json::to_string(&a).unwrap_or_default()),
            step: Some(step),
            metadata: None,
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}

// ============================================================================
/// Session Entity
// ============================================================================

/// Chat session with full history and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID (UUID v4)
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Model used
    pub model: String,
    /// Session title (auto-generated or user-set)
    pub title: Option<String>,
    /// Message history
    pub messages: Vec<Message>,
    /// Session metadata (JSON encoded)
    pub metadata: Option<String>,
    /// Created timestamp
    pub created_at: String,
    /// Last updated timestamp
    pub updated_at: String,
    /// Total token usage
    pub tokens_used: u64,
    /// Total steps in agent flow
    pub total_steps: u32,
    /// Tools used in this session
    pub tools_used: Vec<String>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: impl Into<String>, model: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            model: model.into(),
            title: None,
            messages: Vec::new(),
            metadata: None,
            created_at: now.clone(),
            updated_at: now,
            tokens_used: 0,
            total_steps: 0,
            tools_used: Vec::new(),
        }
    }

    /// Add a message
    pub fn add_message(&mut self, message: Message) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.messages.push(message);
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get latest message
    pub fn latest_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Set title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    /// Add token usage
    pub fn add_tokens(&mut self, tokens: u64) {
        self.tokens_used += tokens;
    }

    /// Record tool usage
    pub fn record_tool(&mut self, tool_name: &str, step: u32) {
        self.total_steps = step;
        if !self.tools_used.contains(&tool_name.to_string()) {
            self.tools_used.push(tool_name.to_string());
        }
    }

    /// Get session summary (last N messages)
    pub fn get_summary(&self, count: usize) -> Vec<Message> {
        let len = self.messages.len();
        if len <= count {
            self.messages.clone()
        } else {
            self.messages[len - count..].to_vec()
        }
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.total_steps = 0;
        self.tools_used.clear();
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}

// ============================================================================
/// Session Summary
// ============================================================================

/// Lightweight session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub user_id: String,
    pub model: String,
    pub title: Option<String>,
    pub message_count: usize,
    pub total_steps: u32,
    pub tools_used: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            user_id: session.user_id.clone(),
            model: session.model.clone(),
            title: session.title.clone(),
            message_count: session.messages.len(),
            total_steps: session.total_steps,
            tools_used: session.tools_used.clone(),
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
        }
    }
}
