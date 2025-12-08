//! Message adapters - convert between different API formats

use crate::types::ChatMessage;
use serde_json::{Value, json};

/// Adapter for converting messages to different API formats
pub struct MessageAdapter;

impl MessageAdapter {
    /// Convert messages to OpenAI-style format
    /// Returns: [{"role": "...", "content": "..."}]
    pub fn to_openai_format(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role.as_str(),
                    "content": msg.content.clone()
                })
            })
            .collect()
    }

    /// Convert messages to Ollama format
    /// Same as OpenAI but simpler structure
    pub fn to_ollama_format(messages: &[ChatMessage]) -> Vec<Value> {
        Self::to_openai_format(messages)
    }

    /// Convert messages to Gemini format
    /// Returns: (system_instruction_text, contents)
    pub fn to_gemini_format(messages: &[ChatMessage]) -> (Option<String>, Vec<Value>) {
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();

        for message in messages {
            match message.role.as_str() {
                "system" => system_parts.push(message.content.clone()),
                "user" => contents.push(json!({
                    "role": "user",
                    "parts": [{"text": message.content.clone()}]
                })),
                "assistant" => contents.push(json!({
                    "role": "model",
                    "parts": [{"text": message.content.clone()}]
                })),
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
