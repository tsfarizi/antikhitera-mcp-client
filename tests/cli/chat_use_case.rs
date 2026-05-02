use antikythera_cli::domain::entities::{
    ChatSession, MessageRole, ProviderConfig, ProviderType, ToolCall, ToolResult,
};
use antikythera_cli::domain::use_cases::chat_use_case::{
    ChatUseCase, LlmProvider, ToolExecutor, parse_agent_action,
};
use antikythera_cli::error::{CliError, CliResult};
use antikythera_core::domain::entities::{AgentAction, Message};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

struct EchoProvider {
    response: String,
}

#[async_trait]
impl LlmProvider for EchoProvider {
    async fn call(&self, _messages: &[Message], _system: &str) -> CliResult<String> {
        Ok(self.response.clone())
    }
}

struct FailingProvider;

#[async_trait]
impl LlmProvider for FailingProvider {
    async fn call(&self, _messages: &[Message], _system: &str) -> CliResult<String> {
        Err(CliError::Validation("LLM unavailable".to_string()))
    }
}

struct NoopExecutor;

#[async_trait]
impl ToolExecutor for NoopExecutor {
    async fn execute(&self, tool_call: &ToolCall) -> CliResult<ToolResult> {
        Ok(ToolResult {
            name: tool_call.name.clone(),
            success: true,
            output: serde_json::json!({"echo": tool_call.arguments}),
            error: None,
        })
    }
}

fn provider_config() -> ProviderConfig {
    ProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        model: "llama3".to_string(),
    }
}

fn session(agent_mode: bool) -> ChatSession {
    let mut s = ChatSession::new(provider_config());
    s.agent_mode = agent_mode;
    s
}

#[tokio::test]
async fn simple_chat_returns_llm_response() {
    let mut uc = ChatUseCase::new(
        session(false),
        Box::new(EchoProvider {
            response: "hello world".to_string(),
        }),
        Box::new(NoopExecutor),
        "system".to_string(),
    );
    let result = uc.send_message("hi").await.expect("should succeed");
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn simple_chat_appends_user_and_assistant_messages() {
    let mut uc = ChatUseCase::new(
        session(false),
        Box::new(EchoProvider {
            response: "reply".to_string(),
        }),
        Box::new(NoopExecutor),
        String::new(),
    );
    uc.send_message("question").await.expect("ok");
    assert_eq!(uc.session.messages.len(), 2);
    assert_eq!(uc.session.messages[0].role, MessageRole::User);
    assert_eq!(uc.session.messages[1].role, MessageRole::Assistant);
    assert_eq!(uc.session.messages[1].content, "reply");
}

#[tokio::test]
async fn simple_chat_propagates_provider_error() {
    let mut uc = ChatUseCase::new(
        session(false),
        Box::new(FailingProvider),
        Box::new(NoopExecutor),
        String::new(),
    );
    let result = uc.send_message("hi").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn agent_loop_returns_final_response() {
    let final_json = r#"{"action":"final","response":"done"}"#;
    let mut uc = ChatUseCase::new(
        session(true),
        Box::new(EchoProvider {
            response: final_json.to_string(),
        }),
        Box::new(NoopExecutor),
        String::new(),
    );
    let result = uc.send_message("task").await.expect("should succeed");
    assert_eq!(result, "done");
}

#[tokio::test]
async fn agent_loop_executes_tool_and_continues_to_final() {
    struct SequentialProvider {
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl LlmProvider for SequentialProvider {
        async fn call(&self, _msgs: &[Message], _sys: &str) -> CliResult<String> {
            let mut calls = self.calls.lock().unwrap();
            let idx = calls.len();
            let resp = if idx == 0 {
                r#"{"action":"call_tool","tool":"echo","input":{"text":"ping"}}"#
            } else {
                r#"{"action":"final","response":"pong"}"#
            };
            calls.push(resp.to_string());
            Ok(resp.to_string())
        }
    }

    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut uc = ChatUseCase::new(
        session(true),
        Box::new(SequentialProvider {
            calls: calls.clone(),
        }),
        Box::new(NoopExecutor),
        String::new(),
    );
    let result = uc.send_message("run").await.expect("should succeed");
    assert_eq!(result, "pong");
    assert_eq!(calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn agent_loop_stops_at_max_steps() {
    let tool_call_json = r#"{"action":"call_tool","tool":"echo","input":{}}"#;
    let mut uc = ChatUseCase::new(
        session(true),
        Box::new(EchoProvider {
            response: tool_call_json.to_string(),
        }),
        Box::new(NoopExecutor),
        String::new(),
    );
    uc.session.max_steps = 2;
    let result = uc.send_message("infinite").await.expect("should not panic");
    assert!(result.contains("exceeded") || result.contains("maximum"));
}

#[test]
fn parse_agent_action_parses_final_response() {
    let json = r#"{"action":"final","response":"all done"}"#;
    match parse_agent_action(json).expect("valid json") {
        AgentAction::FinalResponse(s) => assert_eq!(s, "all done"),
        other => panic!("expected FinalResponse, got {:?}", other),
    }
}

#[test]
fn parse_agent_action_parses_call_tool_with_input() {
    let json = r#"{"action":"call_tool","tool":"search","input":{"q":"rust"}}"#;
    match parse_agent_action(json).expect("valid json") {
        AgentAction::CallTool(call) => {
            assert_eq!(call.name, "search");
            assert_eq!(call.arguments["q"], "rust");
        }
        other => panic!("expected CallTool, got {:?}", other),
    }
}

#[test]
fn parse_agent_action_parses_call_tool_with_arguments_alias() {
    let json = r#"{"action":"call_tool","tool":"greet","arguments":{"name":"Alice"}}"#;
    match parse_agent_action(json).expect("valid json") {
        AgentAction::CallTool(call) => {
            assert_eq!(call.name, "greet");
            assert_eq!(call.arguments["name"], "Alice");
        }
        other => panic!("expected CallTool, got {:?}", other),
    }
}

#[test]
fn parse_agent_action_returns_error_for_invalid_json() {
    assert!(parse_agent_action("not-json").is_err());
}

#[test]
fn parse_agent_action_returns_error_for_missing_action_field() {
    assert!(parse_agent_action(r#"{"response":"missing action"}"#).is_err());
}

#[test]
fn parse_agent_action_returns_error_for_unknown_action() {
    assert!(parse_agent_action(r#"{"action":"unknown","data":{}}"#).is_err());
}

#[test]
fn parse_agent_action_call_tool_defaults_input_to_empty_object_when_absent() {
    let json = r#"{"action":"call_tool","tool":"ping"}"#;
    match parse_agent_action(json).expect("valid") {
        AgentAction::CallTool(call) => assert_eq!(call.arguments, serde_json::json!({})),
        other => panic!("expected CallTool, got {:?}", other),
    }
}
