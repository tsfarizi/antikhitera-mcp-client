//! Ollama provider compatibility shim.
//!
//! Direct model calls are disabled. The embedding host must perform the model
//! request and return the result through the framework boundary.

use crate::domain::use_cases::chat_use_case::LlmProvider;
use async_trait::async_trait;
use std::error::Error;

pub struct OllamaProvider {
    model: String,
    #[allow(dead_code)]
    endpoint: String,
}

impl OllamaProvider {
    /// Create an Ollama provider pointing at the default local endpoint.
    pub fn new(model: String) -> Self {
        let endpoint = "http://127.0.0.1:11434".to_string();
        Self::with_endpoint_inner(model, endpoint)
    }

    fn with_endpoint_inner(model: String, endpoint: String) -> Self {
        Self {
            model,
            endpoint,
        }
    }

    /// Override the Ollama server URL (e.g. for a remote instance).
    pub fn with_endpoint(self, endpoint: String) -> Self {
        Self::with_endpoint_inner(self.model, endpoint)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn call(
        &self,
        _messages: &[crate::domain::entities::Message],
        _system_prompt: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        Err(std::io::Error::other(format!(
            "Direct Ollama model invocation is disabled for model '{}' at '{}'. The host must perform the call and hand the response back to the framework.",
            self.model,
            self.endpoint
        ))
        .into())
    }
}

