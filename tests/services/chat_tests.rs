use antikythera_core::application::client::{ClientConfig, McpClient};
use antikythera_core::application::model_provider::ModelProvider;
use antikythera_core::application::services::chat::ChatService;
use antikythera_core::domain::types::ChatMessage;
use antikythera_core::domain::types::MessageRole;
use antikythera_core::infrastructure::model::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

struct MockModelProvider {
    response_content: String,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ModelProvider for MockModelProvider {
    async fn chat(&self, _request: ModelRequest) -> Result<ModelResponse, ModelError> {
        Ok(ModelResponse {
            message: ChatMessage::new(
                MessageRole::Assistant,
                self.response_content.clone(),
            ),
            session_id: None,
            tokens: 0,
        })
    }
}

#[tokio::test]
async fn test_chat_service_raw_mode_no_debug() {
    let provider = MockModelProvider {
        response_content: "Hello world".to_string(),
    };
    let config = ClientConfig::new("test-provider", "test-model");
    let client = Arc::new(McpClient::new(provider, config));
    let service = ChatService::new(client);

    let result = service
        .process_request(
            "hi".to_string(),
            vec![],
            None,
            None,
            false, // agent disabled
            None,
            false, // debug disabled
        )
        .await;

    assert!(result.is_ok());
    let outcome = result.unwrap();
    assert_eq!(outcome.content, json!("Hello world"));
    assert!(outcome.logs.is_none());
    assert!(outcome.provider.is_none());
}

#[tokio::test]
async fn test_chat_service_raw_mode_with_debug() {
    let provider = MockModelProvider {
        response_content: "Hello world".to_string(),
    };
    let config = ClientConfig::new("test-provider", "test-model");
    let client = Arc::new(McpClient::new(provider, config));
    let service = ChatService::new(client);

    let result = service
        .process_request(
            "hi".to_string(),
            vec![],
            None,
            None,
            false, // agent disabled
            None,
            true, // debug enabled
        )
        .await;

    assert!(result.is_ok());
    let outcome = result.unwrap();
    assert_eq!(outcome.content, json!("Hello world"));
    assert!(outcome.logs.is_some());
    assert!(outcome.provider.is_some());
    assert_eq!(outcome.provider.unwrap(), "test-provider");
}
