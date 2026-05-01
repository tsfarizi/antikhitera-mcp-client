pub use antikythera_session::{MessagePart, MessageRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
}

impl ChatMessage {
    /// Create a new text-only message (backwards compatible)
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![MessagePart::text(content)],
        }
    }

    /// Create a message with multiple parts
    pub fn with_parts(role: MessageRole, parts: Vec<MessagePart>) -> Self {
        Self { role, parts }
    }

    /// Get the text content of the message (concatenated from all text parts)
    pub fn content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| p.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this message contains any non-text parts
    pub fn has_attachments(&self) -> bool {
        self.parts
            .iter()
            .any(|p| !matches!(p, MessagePart::Text { .. }))
    }
}
