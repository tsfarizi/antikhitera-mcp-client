//! Model traits

use super::types::{ModelError, ModelRequest, ModelResponse, ModelStreamEvent};
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;

/// Trait for model provider implementations
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ModelProvider: Send + Sync {
    /// Send a chat request to the model provider
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;

    /// Stream a chat request to the model provider.
    async fn chat_stream(
        &self,
        request: ModelRequest,
        sender: UnboundedSender<ModelStreamEvent>,
    ) -> Result<ModelResponse, ModelError> {
        let response = self.chat(request.clone()).await?;
        let _ = sender.send(ModelStreamEvent::Started {
            provider: request.provider,
            model: request.model,
            session_id: response.session_id.clone(),
            correlation_id: response.correlation_id.clone(),
        });
        let content = response.message.content();
        if !content.is_empty() {
            let _ = sender.send(ModelStreamEvent::TextDelta { delta: content });
        }
        for tool_call in &response.tool_calls {
            let _ = sender.send(ModelStreamEvent::ToolCall {
                tool_call: tool_call.clone(),
            });
        }
        let _ = sender.send(ModelStreamEvent::Finished {
            finish_reason: response.finish_reason.clone(),
        });
        Ok(response)
    }
}

/// Trait for individual model clients
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ModelClient: Send + Sync {
    /// Get the client ID
    fn id(&self) -> &str;

    /// Send a chat request
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;

    /// Stream a chat request to the model provider.
    async fn chat_stream(
        &self,
        request: ModelRequest,
        sender: UnboundedSender<ModelStreamEvent>,
    ) -> Result<ModelResponse, ModelError> {
        let response = self.chat(request.clone()).await?;
        let _ = sender.send(ModelStreamEvent::Started {
            provider: request.provider,
            model: request.model,
            session_id: response.session_id.clone(),
            correlation_id: response.correlation_id.clone(),
        });
        let content = response.message.content();
        if !content.is_empty() {
            let _ = sender.send(ModelStreamEvent::TextDelta { delta: content });
        }
        for tool_call in &response.tool_calls {
            let _ = sender.send(ModelStreamEvent::ToolCall {
                tool_call: tool_call.clone(),
            });
        }
        let _ = sender.send(ModelStreamEvent::Finished {
            finish_reason: response.finish_reason.clone(),
        });
        Ok(response)
    }
}
