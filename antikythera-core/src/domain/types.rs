use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "system" => Some(MessageRole::System),
            "user" => Some(MessageRole::User),
            "assistant" => Some(MessageRole::Assistant),
            _ => None,
        }
    }
}

/// A part of a message content - can be text, image, or file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        text: String,
    },
    Image {
        mime_type: String,
        data: String,
    },
    File {
        name: String,
        mime_type: String,
        data: String,
    },
}

impl MessagePart {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    pub fn image(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            mime_type: mime_type.into(),
            data: data.into(),
        }
    }

    pub fn file(
        name: impl Into<String>,
        mime_type: impl Into<String>,
        data: impl Into<String>,
    ) -> Self {
        Self::File {
            name: name.into(),
            mime_type: mime_type.into(),
            data: data.into(),
        }
    }

    /// Get text content if this is a text part
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

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
