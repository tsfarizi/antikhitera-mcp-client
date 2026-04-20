//! Gemini provider compatibility shim.
//!
//! Direct model calls are disabled. The embedding host must perform the model
//! request and return the result through the framework boundary.

use crate::domain::use_cases::chat_use_case::LlmProvider;
use crate::error::{CliError, CliResult};
use async_trait::async_trait;

pub struct GeminiProvider {
    #[allow(dead_code)]
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn call(
        &self,
        _messages: &[crate::domain::entities::Message],
        _system_prompt: &str,
    ) -> CliResult<String> {
        Err(CliError::Unsupported(format!(
            "Direct Gemini model invocation is disabled for model '{}'. The host must perform the call and hand the response back to the framework.",
            self.model
        )))
    }
}
