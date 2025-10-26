use super::runtime::ToolRuntime;
use super::*;
use crate::application::client::ClientConfig;
use crate::application::tooling::{ServerToolInfo, ToolInvokeError, ToolServerInterface};
use crate::client::McpClient;
use crate::config::ToolConfig;
use crate::model::{ModelError, ModelProvider, ModelRequest, ModelResponse};
use crate::types::{ChatMessage, MessageRole};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct StubBridge {
    result: Value,
    instruction: Option<String>,
    metadata: Option<ServerToolInfo>,
}

#[async_trait]
impl ToolServerInterface for StubBridge {
    async fn invoke_tool(
        &self,
        _server: &str,
        _tool: &str,
        _arguments: Value,
    ) -> Result<Value, ToolInvokeError> {
        Ok(self.result.clone())
    }

    async fn server_instructions(&self, _server: &str) -> Option<String> {
        self.instruction.clone()
    }

    async fn tool_metadata(&self, _server: &str, _tool: &str) -> Option<ServerToolInfo> {
        self.metadata.clone()
    }
}

#[derive(Clone)]
struct ScriptedProvider {
    responses: Arc<Mutex<Vec<String>>>,
    recordings: Arc<Mutex<Vec<ModelRequest>>>,
}

impl ScriptedProvider {
    fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(
                responses.into_iter().map(String::from).collect(),
            )),
            recordings: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn requests(&self) -> Vec<ModelRequest> {
        self.recordings.lock().await.clone()
    }
}

#[async_trait]
impl ModelProvider for ScriptedProvider {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let mut responses = self.responses.lock().await;
        let response = responses.remove(0);
        let mut recordings = self.recordings.lock().await;
        recordings.push(request.clone());
        Ok(ModelResponse {
            message: ChatMessage::new(MessageRole::Assistant, response),
            session_id: request.session_id,
        })
    }
}

#[tokio::test]
async fn agent_returns_final_response_without_tools() {
    let provider = ScriptedProvider::new(vec![r#"{"action":"final","response":"done"}"#]);
    let client = McpClient::new(provider.clone(), ClientConfig::new("llama"));
    let agent = Agent::new(Arc::new(client));

    let outcome = agent
        .run("hello world".into(), AgentOptions::default())
        .await
        .expect("agent succeeds");

    assert_eq!(outcome.response, "done");
    assert!(outcome.steps.is_empty());

    let records = provider.requests().await;
    assert!(!records.is_empty());
    let first_request = &records[0];
    assert!(
        first_request
            .messages
            .iter()
            .any(|msg| msg.content.contains("hello world"))
    );
    assert!(
        first_request
            .messages
            .iter()
            .all(|msg| !msg.content.contains("tool_context"))
    );
}

#[tokio::test]
async fn agent_handles_list_tools() {
    let provider = ScriptedProvider::new(vec![
        r#"{"action":"call_tool","tool":"list_tools"}"#,
        r#"{"action":"final","response":"all done"}"#,
    ]);
    let mut cfg = ClientConfig::new("llama");
    cfg = cfg.with_tools(vec![
        ToolConfig {
            name: "weather".into(),
            description: Some("Fetch weather.".into()),
            server: Some("utilities".into()),
        },
        ToolConfig {
            name: "search".into(),
            description: None,
            server: Some("utilities".into()),
        },
    ]);
    let client = McpClient::new(provider.clone(), cfg);
    let agent = Agent::new(Arc::new(client));

    let outcome = agent
        .run("need info".into(), AgentOptions::default())
        .await
        .expect("agent succeeds");

    assert_eq!(outcome.response, "all done");
    assert_eq!(outcome.steps.len(), 1);
    assert_eq!(outcome.steps[0].tool, "list_tools");
    assert!(outcome.steps[0].success);
    assert!(
        outcome.steps[0]
            .output
            .get("tools")
            .and_then(Value::as_array)
            .map(|tools| !tools.is_empty())
            .unwrap_or(false)
    );

    let records = provider.requests().await;
    assert_eq!(records.len(), 2);
    assert!(
        records[0]
            .messages
            .iter()
            .any(|msg| msg.content.contains("\"tool_context\""))
    );
    assert!(
        records[1]
            .messages
            .iter()
            .any(|msg| msg.content.contains("tool_result"))
    );
}

#[tokio::test]
async fn tool_runtime_invokes_executor_and_extracts_message() {
    let configs = vec![ToolConfig {
        name: "get_current_time".into(),
        description: Some("Fetch current time".into()),
        server: Some("time".into()),
    }];

    let payload = json!({
        "content": [
            { "type": "text", "text": "Waktu saat ini 10:00" }
        ],
        "isError": false
    });

    let bridge = Arc::new(StubBridge {
        result: payload.clone(),
        instruction: Some("Selalu gunakan tool untuk memastikan waktu akurat".into()),
        metadata: Some(ServerToolInfo {
            name: "get_current_time".into(),
            description: Some("Ambil waktu terkini".into()),
            input_schema: Some(json!({"type":"object"})),
        }),
    });
    let runtime = ToolRuntime::new(configs, bridge);

    let execution = runtime
        .execute("get_current_time", Value::Null)
        .await
        .expect("tool execution succeeds");

    assert!(execution.success);
    assert_eq!(execution.tool, "get_current_time");
    assert_eq!(execution.output, payload);
    assert_eq!(execution.message.as_deref(), Some("Waktu saat ini 10:00"));
}
