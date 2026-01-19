//! Message adapters - convert between different API formats

use crate::types::{ChatMessage, MessagePart};
use serde_json::{Value, json};

/// Adapter for converting messages to different API formats
pub struct MessageAdapter;

impl MessageAdapter {
    /// Convert a MessagePart to Gemini format
    fn part_to_gemini(part: &MessagePart) -> Value {
        match part {
            MessagePart::Text { text } => json!({"text": text}),
            MessagePart::Image { mime_type, data } => json!({
                "inline_data": {
                    "mime_type": mime_type,
                    "data": data
                }
            }),
            MessagePart::File {
                name: _,
                mime_type,
                data,
            } => json!({
                "inline_data": {
                    "mime_type": mime_type,
                    "data": data
                }
            }),
        }
    }

    /// Convert a MessagePart to OpenAI format
    fn part_to_openai(part: &MessagePart) -> Value {
        match part {
            MessagePart::Text { text } => json!({
                "type": "text",
                "text": text
            }),
            MessagePart::Image { mime_type, data } => json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", mime_type, data)
                }
            }),
            MessagePart::File {
                name: _,
                mime_type,
                data,
            } => json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", mime_type, data)
                }
            }),
        }
    }

    /// Convert messages to OpenAI-style format
    /// Returns: [{"role": "...", "content": [...]}]
    pub fn to_openai_format(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                // If message has only text parts, use simple string format
                let all_text = msg
                    .parts
                    .iter()
                    .all(|p| matches!(p, MessagePart::Text { .. }));

                if all_text {
                    json!({
                        "role": msg.role.as_str(),
                        "content": msg.content()
                    })
                } else {
                    json!({
                        "role": msg.role.as_str(),
                        "content": msg.parts.iter().map(Self::part_to_openai).collect::<Vec<_>>()
                    })
                }
            })
            .collect()
    }

    /// Convert messages to Ollama format
    /// Same as OpenAI but simpler structure
    pub fn to_ollama_format(messages: &[ChatMessage]) -> Vec<Value> {
        // Ollama uses simple text format, extract content only
        messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role.as_str(),
                    "content": msg.content()
                })
            })
            .collect()
    }

    /// Convert messages to Gemini format
    /// Returns: (system_instruction_text, contents)
    pub fn to_gemini_format(messages: &[ChatMessage]) -> (Option<String>, Vec<Value>) {
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();

        for message in messages {
            match message.role.as_str() {
                "system" => system_parts.push(message.content()),
                "user" => {
                    let parts: Vec<Value> =
                        message.parts.iter().map(Self::part_to_gemini).collect();
                    contents.push(json!({
                        "role": "user",
                        "parts": parts
                    }));
                }
                "assistant" => {
                    let parts: Vec<Value> =
                        message.parts.iter().map(Self::part_to_gemini).collect();
                    contents.push(json!({
                        "role": "model",
                        "parts": parts
                    }));
                }
                _ => {}
            }
        }

        let system_instruction = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        (system_instruction, contents)
    }
}
