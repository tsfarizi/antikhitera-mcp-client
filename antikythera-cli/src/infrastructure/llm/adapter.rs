//! Message-format adapters — CLI's own copy for LLM provider calls
//!
//! Converts `antikythera_core::domain::types::ChatMessage` messages into the
//! wire formats expected by each LLM provider API.  This module is the
//! **CLI-side** home of these adapters; having them here (rather than in
//! `antikythera-core`) keeps the WASM component target free of API-specific
//! serialisation logic.

use antikythera_core::domain::types::{ChatMessage, MessagePart};
use serde_json::{Value, json};

/// Adapter for converting core `ChatMessage` instances to provider wire formats.
pub struct MessageAdapter;

impl MessageAdapter {
    // ── Internal helpers ────────────────────────────────────────────────────

    fn part_to_gemini(part: &MessagePart) -> Value {
        match part {
            MessagePart::Text { text } => json!({"text": text}),
            MessagePart::Image { mime_type, data } => json!({
                "inline_data": { "mime_type": mime_type, "data": data }
            }),
            MessagePart::File {
                mime_type, data, ..
            } => json!({
                "inline_data": { "mime_type": mime_type, "data": data }
            }),
        }
    }

    fn part_to_openai(part: &MessagePart) -> Value {
        match part {
            MessagePart::Text { text } => json!({"type": "text", "text": text}),
            MessagePart::Image { mime_type, data } => json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{};base64,{}", mime_type, data) }
            }),
            MessagePart::File {
                mime_type, data, ..
            } => json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{};base64,{}", mime_type, data) }
            }),
        }
    }

    // ── Public converters ────────────────────────────────────────────────────

    /// Convert messages to OpenAI-compatible format.
    ///
    /// Returns `[{"role": "...", "content": "..."}]`.
    /// Multi-part messages are returned as an array of content objects.
    pub fn to_openai_format(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let all_text = msg
                    .parts
                    .iter()
                    .all(|p| matches!(p, MessagePart::Text { .. }));

                if all_text {
                    json!({"role": msg.role.as_str(), "content": msg.content()})
                } else {
                    json!({
                        "role": msg.role.as_str(),
                        "content": msg.parts.iter().map(Self::part_to_openai).collect::<Vec<_>>()
                    })
                }
            })
            .collect()
    }

    /// Convert messages to Ollama format (simplified OpenAI-like structure).
    pub fn to_ollama_format(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| json!({"role": msg.role.as_str(), "content": msg.content()}))
            .collect()
    }

    /// Convert messages to Gemini format.
    ///
    /// Returns `(system_instruction_text, contents)`.
    /// System messages are extracted into the first return value; all other
    /// messages are placed in `contents` with `"user"` / `"model"` roles.
    pub fn to_gemini_format(messages: &[ChatMessage]) -> (Option<String>, Vec<Value>) {
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();

        for message in messages {
            match message.role.as_str() {
                "system" => system_parts.push(message.content()),
                "user" => {
                    let parts: Vec<Value> =
                        message.parts.iter().map(Self::part_to_gemini).collect();
                    contents.push(json!({"role": "user", "parts": parts}));
                }
                "assistant" => {
                    let parts: Vec<Value> =
                        message.parts.iter().map(Self::part_to_gemini).collect();
                    contents.push(json!({"role": "model", "parts": parts}));
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
