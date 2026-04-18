use std::sync::Arc;

use antikythera_core::application::agent::multi_agent::{
    execution::ExecutionMode,
    orchestrator::MultiAgentOrchestrator,
    registry::AgentProfile,
    task::AgentTask,
};
use antikythera_core::{ChatRequest, ClientConfig, McpClient, ModelProvider, ModelStreamEvent};
use antikythera_core::infrastructure::model::{ModelError, ModelRequest, ModelResponse};
use async_trait::async_trait;
use tokio::sync::mpsc::unbounded_channel;

#[derive(Clone, Default)]
struct FakeProvider;

#[async_trait]
impl ModelProvider for FakeProvider {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let prompt = request
            .messages
            .last()
            .map(|message| message.content())
            .unwrap_or_default();

        if prompt.contains("Current task:") {
            Ok(ModelResponse::new(
                "{\"action\":\"final\",\"response\":\"pipeline-ok\"}".to_string(),
                request.session_id,
            ))
        } else {
            Ok(ModelResponse::new(
                "{\"action\":\"final\",\"response\":\"ok\"}".to_string(),
                request.session_id,
            ))
        }
    }
}

fn make_client() -> Arc<McpClient<FakeProvider>> {
    let config = ClientConfig::new("fake", "demo-model");
    Arc::new(McpClient::new(FakeProvider, config))
}

#[tokio::test]
async fn chat_stream_emits_default_fallback_events() {
    let client = make_client();
    let (sender, mut receiver) = unbounded_channel();

    let result = client
        .chat_stream(
            ChatRequest {
                prompt: "hello stream".to_string(),
                attachments: Vec::new(),
                system_prompt: None,
                session_id: None,
                raw_mode: false,
                bypass_template: false,
                force_json: false,
                correlation_id: Some("corr-stream".to_string()),
                tools: Vec::new(),
                tool_choice: None,
            },
            sender,
        )
        .await
        .unwrap();

    assert_eq!(result.correlation_id, "corr-stream");

    let mut event_types = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        match event {
            ModelStreamEvent::Started { .. } => event_types.push("started"),
            ModelStreamEvent::TextDelta { .. } => event_types.push("delta"),
            ModelStreamEvent::Finished { .. } => event_types.push("finished"),
            ModelStreamEvent::ToolCall { .. } => event_types.push("tool"),
        }
    }

    assert_eq!(event_types, vec!["started", "delta", "finished"]);
}

#[tokio::test]
async fn multi_agent_dispatch_and_pipeline_succeed() {
    let client = make_client();
    let orchestrator = MultiAgentOrchestrator::new(client, ExecutionMode::Sequential)
        .register_agent(AgentProfile {
            id: "reviewer".to_string(),
            name: "Reviewer".to_string(),
            role: "review".to_string(),
            system_prompt: Some("You are a reviewer.".to_string()),
            max_steps: Some(4),
        });

    let single = orchestrator
        .dispatch(AgentTask::new("Review this module"))
        .await;
    assert!(single.success);
    assert_eq!(single.agent_id, "reviewer");

    let pipeline = orchestrator
        .pipeline(vec![
            AgentTask::new("Step one"),
            AgentTask::new("Step two"),
        ])
        .await;

    assert!(pipeline.success);
    assert_eq!(pipeline.task_results.len(), 2);
    assert_eq!(pipeline.final_output, serde_json::Value::String("pipeline-ok".to_string()));
}
