//! OpenAI-compatible client implementation

use async_trait::async_trait;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tracing::{debug, info};

use super::base::HttpClientBase;
use crate::config::ModelProviderConfig;
use crate::infrastructure::model::adapter::MessageAdapter;
use crate::infrastructure::model::factory::resolve_api_key;
use crate::infrastructure::model::traits::ModelClient;
use crate::infrastructure::model::types::{
    ModelError, ModelRequest, ModelResponse, ModelStreamEvent, ModelToolCall,
};

/// OpenAI-compatible client (works with OpenAI, Anthropic, Mistral, Groq, etc.)
#[derive(Clone)]
pub struct OpenAIClient {
    base: HttpClientBase,
    api_path: String,
}

impl OpenAIClient {
    pub fn from_config(config: &ModelProviderConfig) -> Self {
        let api_key = resolve_api_key(&config.id, config.api_key.as_deref());
        Self {
            base: HttpClientBase::new(config.id.clone(), config.endpoint.clone(), api_key),
            api_path: config
                .api_path
                .clone()
                .unwrap_or_else(|| "/v1/chat/completions".to_string()),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ModelClient for OpenAIClient {
    fn id(&self) -> &str {
        &self.base.id
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.base.build_url(&self.api_path);

        let payload = OpenAIRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_openai_format(&request.messages),
            stream: false,
            tools: if request.tools.is_empty() {
                None
            } else {
                Some(MessageAdapter::to_openai_tools(&request.tools))
            },
            tool_choice: request
                .tool_choice
                .as_ref()
                .map(MessageAdapter::to_openai_tool_choice),
        };

        info!(
            provider = self.base.id.as_str(),
            model = request.model.as_str(),
            messages = request.messages.len(),
            "Sending request to OpenAI-compatible provider"
        );

        let response: OpenAIResponse = self.base.post_with_bearer(&url, &payload).await?;
        debug!("Received response from OpenAI-compatible provider");

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing choice"))?;

        let message = choice
            .message
            .ok_or_else(|| ModelError::invalid_response(&self.base.id, "missing message"))?;
        let content = message.content.unwrap_or_default();
        let tool_calls = parse_tool_calls(message.tool_calls);

        Ok(ModelResponse::with_details(
            content,
            request.session_id,
            request.correlation_id,
            tool_calls,
            choice.finish_reason,
        ))
    }

    async fn chat_stream(
        &self,
        request: ModelRequest,
        sender: UnboundedSender<ModelStreamEvent>,
    ) -> Result<ModelResponse, ModelError> {
        let api_key = self
            .base
            .api_key
            .as_deref()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| ModelError::missing_api_key(&self.base.id))?;
        let url = self.base.build_url(&self.api_path);
        let payload = OpenAIRequest {
            model: request.model.clone(),
            messages: MessageAdapter::to_openai_format(&request.messages),
            stream: true,
            tools: if request.tools.is_empty() {
                None
            } else {
                Some(MessageAdapter::to_openai_tools(&request.tools))
            },
            tool_choice: request
                .tool_choice
                .as_ref()
                .map(MessageAdapter::to_openai_tool_choice),
        };

        let _ = sender.send(ModelStreamEvent::Started {
            provider: request.provider.clone(),
            model: request.model.clone(),
            session_id: request.session_id.clone(),
            correlation_id: request.correlation_id.clone(),
        });

        let request_builder = self
            .base
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload);
        let mut events = request_builder
            .eventsource()
            .map_err(|e| ModelError::network(&self.base.id, e.to_string()))?;

        let mut content = String::new();
        let mut finish_reason = None;
        let mut tool_calls = Vec::new();

        while let Some(event) = events.next().await {
            match event {
                Ok(Event::Open) => {}
                Ok(Event::Message(message)) => {
                    let data = message.data.trim();
                    if data == "[DONE]" {
                        break;
                    }

                    let chunk: OpenAIStreamResponse = serde_json::from_str(data)
                        .map_err(|e| ModelError::invalid_response(&self.base.id, e.to_string()))?;

                    for choice in chunk.choices {
                        if let Some(reason) = choice.finish_reason {
                            finish_reason = Some(reason);
                        }
                        if let Some(delta) = choice.delta {
                            if let Some(text) = delta.content {
                                content.push_str(&text);
                                let _ = sender.send(ModelStreamEvent::TextDelta { delta: text });
                            }
                            if let Some(delta_tool_calls) = delta.tool_calls {
                                merge_tool_call_chunks(&mut tool_calls, delta_tool_calls);
                            }
                        }
                    }
                }
                Err(err) => return Err(ModelError::network(&self.base.id, err.to_string())),
            }
        }

        let finalized_tool_calls = finalize_tool_calls(tool_calls);
        for tool_call in &finalized_tool_calls {
            let _ = sender.send(ModelStreamEvent::ToolCall {
                tool_call: tool_call.clone(),
            });
        }
        let _ = sender.send(ModelStreamEvent::Finished {
            finish_reason: finish_reason.clone(),
        });

        Ok(ModelResponse::with_details(
            content,
            request.session_id,
            request.correlation_id,
            finalized_tool_calls,
            finish_reason,
        ))
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: Option<OpenAIMessage>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAIWireToolCall>,
}

#[derive(Deserialize)]
struct OpenAIWireToolCall {
    id: Option<String>,
    function: OpenAIWireFunction,
}

#[derive(Deserialize)]
struct OpenAIWireFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: Option<OpenAIStreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIWireToolCallChunk>>,
}

#[derive(Deserialize)]
struct OpenAIWireToolCallChunk {
    index: usize,
    id: Option<String>,
    function: Option<OpenAIWireFunctionChunk>,
}

#[derive(Deserialize)]
struct OpenAIWireFunctionChunk {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Default)]
struct AccumulatedToolCall {
    id: Option<String>,
    name: String,
    arguments: String,
}

fn parse_tool_calls(tool_calls: Vec<OpenAIWireToolCall>) -> Vec<ModelToolCall> {
    tool_calls
        .into_iter()
        .map(|tool_call| ModelToolCall {
            id: tool_call.id,
            name: tool_call.function.name,
            arguments: parse_tool_arguments(&tool_call.function.arguments),
        })
        .collect()
}

fn merge_tool_call_chunks(
    accumulated: &mut Vec<AccumulatedToolCall>,
    chunks: Vec<OpenAIWireToolCallChunk>,
) {
    for chunk in chunks {
        if accumulated.len() <= chunk.index {
            accumulated.resize_with(chunk.index + 1, AccumulatedToolCall::default);
        }

        let current = &mut accumulated[chunk.index];
        if chunk.id.is_some() {
            current.id = chunk.id;
        }
        if let Some(function) = chunk.function {
            if let Some(name) = function.name {
                current.name.push_str(&name);
            }
            if let Some(arguments) = function.arguments {
                current.arguments.push_str(&arguments);
            }
        }
    }
}

fn finalize_tool_calls(accumulated: Vec<AccumulatedToolCall>) -> Vec<ModelToolCall> {
    accumulated
        .into_iter()
        .filter(|tool_call| !tool_call.name.is_empty())
        .map(|tool_call| ModelToolCall {
            id: tool_call.id,
            name: tool_call.name,
            arguments: parse_tool_arguments(&tool_call.arguments),
        })
        .collect()
}

fn parse_tool_arguments(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}
