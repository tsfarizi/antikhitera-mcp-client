//! Chat Feature Slice — Domain Use Case
//!
//! This module is the **domain core** of the Chat slice in VSA.
//! It defines the ports (`LlmProvider`, `ToolExecutor`) and the
//! `ChatUseCase` orchestrator that drives the agent loop.
//!
//! No I/O or infrastructure details appear here — callers inject
//! concrete implementations via trait objects (dependency inversion).

use crate::domain::entities::*;
use crate::error::{CliError, CliResult};

/// LLM provider port (dependency injection)
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn call(&self, messages: &[Message], system_prompt: &str) -> CliResult<String>;
}

/// Tool executor port
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, tool_call: &ToolCall) -> CliResult<ToolResult>;
}

/// Chat use case
pub struct ChatUseCase {
    pub session: ChatSession,
    pub llm: Box<dyn LlmProvider>,
    pub tools: Box<dyn ToolExecutor>,
    pub system_prompt: String,
}

impl ChatUseCase {
    pub fn new(
        session: ChatSession,
        llm: Box<dyn LlmProvider>,
        tools: Box<dyn ToolExecutor>,
        system_prompt: String,
    ) -> Self {
        Self {
            session,
            llm,
            tools,
            system_prompt,
        }
    }

    /// Send user message and get response (may involve tool calls)
    pub async fn send_message(&mut self, user_input: &str) -> CliResult<String> {
        // Add user message
        self.session.add_message(Message::user(user_input));

        // Agent mode: loop until final response or max steps
        if self.session.agent_mode {
            self.run_agent_loop().await
        } else {
            // Simple mode: single LLM call
            self.simple_chat().await
        }
    }

    /// Simple chat (no tool calling)
    async fn simple_chat(&mut self) -> CliResult<String> {
        let response = self
            .llm
            .call(&self.session.messages, &self.system_prompt)
            .await?;
        self.session.add_message(Message::assistant(&response));
        Ok(response)
    }

    /// Agent mode: loop with tool calls
    async fn run_agent_loop(&mut self) -> CliResult<String> {
        loop {
            // Check max steps
            if self.session.is_max_steps_exceeded() {
                return Ok("Agent exceeded maximum steps.".to_string());
            }

            // Call LLM
            let response = self
                .llm
                .call(&self.session.messages, &self.system_prompt)
                .await?;

            // Parse action from response
            let action = parse_agent_action(&response)?;

            match action {
                AgentAction::CallTool(tool_call) => {
                    self.session.current_step += 1;

                    // Execute tool
                    let result = self.tools.execute(&tool_call).await?;

                    // Add tool result to messages
                    let tool_msg = if result.success {
                        Message::tool(format!(
                            "Tool '{}' result: {}",
                            result.name,
                            serde_json::to_string_pretty(&result.output).unwrap_or_default()
                        ))
                    } else {
                        Message::tool(format!(
                            "Tool '{}' error: {}",
                            result.name,
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        ))
                    };
                    self.session.add_message(tool_msg);
                }

                AgentAction::FinalResponse(content) => {
                    self.session.add_message(Message::assistant(&content));
                    return Ok(content);
                }

                AgentAction::Error(error) => {
                    return Ok(format!("Agent error: {}", error));
                }
            }
        }
    }
}

/// Parse agent action from LLM response
fn parse_agent_action(response: &str) -> CliResult<AgentAction> {
    // Try parse as JSON
    let value: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| CliError::Validation(format!("Invalid JSON: {}", e)))?;

    let action = value
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::Validation("Missing 'action' field".to_string()))?;

    match action {
        "call_tool" => {
            let tool = value
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::Validation("Missing 'tool' field".to_string()))?
                .to_string();

            let input = value
                .get("input")
                .or_else(|| value.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            Ok(AgentAction::CallTool(ToolCall {
                name: tool,
                arguments: input,
            }))
        }

        "final" => {
            let content = value
                .get("response")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::Validation("Missing 'response' field".to_string()))?
                .to_string();

            Ok(AgentAction::FinalResponse(content))
        }

        _ => Err(CliError::Validation(format!("Unknown action: {}", action))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Mock implementations ─────────────────────────────────────────────────

    struct EchoProvider {
        response: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for EchoProvider {
        async fn call(&self, _messages: &[Message], _system: &str) -> CliResult<String> {
            Ok(self.response.clone())
        }
    }

    struct FailingProvider;

    #[async_trait::async_trait]
    impl LlmProvider for FailingProvider {
        async fn call(&self, _messages: &[Message], _system: &str) -> CliResult<String> {
            Err(CliError::Validation("LLM unavailable".to_string()))
        }
    }

    struct NoopExecutor;

    #[async_trait::async_trait]
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

    // ── simple_chat slice ────────────────────────────────────────────────────

    #[tokio::test]
    async fn simple_chat_returns_llm_response() {
        let mut uc = ChatUseCase::new(
            session(false),
            Box::new(EchoProvider { response: "hello world".to_string() }),
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
            Box::new(EchoProvider { response: "reply".to_string() }),
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

    // ── agent loop slice ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn agent_loop_returns_final_response() {
        let final_json = r#"{"action":"final","response":"done"}"#;
        let mut uc = ChatUseCase::new(
            session(true),
            Box::new(EchoProvider { response: final_json.to_string() }),
            Box::new(NoopExecutor),
            String::new(),
        );

        let result = uc.send_message("task").await.expect("should succeed");
        assert_eq!(result, "done");
    }

    #[tokio::test]
    async fn agent_loop_executes_tool_and_continues_to_final() {
        // First call: tool call JSON; second call: final response.
        use std::sync::{Arc, Mutex};

        struct SequentialProvider {
            calls: Arc<Mutex<Vec<String>>>,
        }

        #[async_trait::async_trait]
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
            Box::new(SequentialProvider { calls: calls.clone() }),
            Box::new(NoopExecutor),
            String::new(),
        );

        let result = uc.send_message("run").await.expect("should succeed");
        assert_eq!(result, "pong");
        // LLM was called twice: once for tool, once for final answer.
        assert_eq!(calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn agent_loop_stops_at_max_steps() {
        // Always return a tool call so the loop never terminates naturally.
        let tool_call_json = r#"{"action":"call_tool","tool":"echo","input":{}}"#;
        let mut uc = ChatUseCase::new(
            session(true),
            Box::new(EchoProvider { response: tool_call_json.to_string() }),
            Box::new(NoopExecutor),
            String::new(),
        );
        uc.session.max_steps = 2;

        let result = uc.send_message("infinite").await.expect("should not panic");
        assert!(result.contains("exceeded") || result.contains("maximum"));
    }

    // ── parse_agent_action slice ─────────────────────────────────────────────

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
        let result = parse_agent_action("not-json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_agent_action_returns_error_for_missing_action_field() {
        let result = parse_agent_action(r#"{"response":"missing action"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn parse_agent_action_returns_error_for_unknown_action() {
        let result = parse_agent_action(r#"{"action":"unknown","data":{}}"#);
        assert!(result.is_err());
    }

    #[test]
    fn parse_agent_action_call_tool_defaults_input_to_empty_object_when_absent() {
        let json = r#"{"action":"call_tool","tool":"ping"}"#;
        match parse_agent_action(json).expect("valid") {
            AgentAction::CallTool(call) => {
                assert_eq!(call.arguments, serde_json::json!({}));
            }
            other => panic!("expected CallTool, got {:?}", other),
        }
    }
}
