//! Chat use case
//!
//! Orchestrates the chat flow: user input → LLM → tool call → response.
//! This is the domain logic - it depends on ports (interfaces), not implementations.

use crate::domain::entities::*;
use std::error::Error;

/// LLM provider port (dependency injection)
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn call(
        &self,
        messages: &[Message],
        system_prompt: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
}

/// Tool executor port
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_call: &ToolCall,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>>;
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
    pub async fn send_message(
        &mut self,
        user_input: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
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
    async fn simple_chat(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self
            .llm
            .call(&self.session.messages, &self.system_prompt)
            .await?;
        self.session.add_message(Message::assistant(&response));
        Ok(response)
    }

    /// Agent mode: loop with tool calls
    async fn run_agent_loop(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
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
fn parse_agent_action(response: &str) -> Result<AgentAction, String> {
    // Try parse as JSON
    let value: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("Invalid JSON: {}", e))?;

    let action = value
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'action' field")?;

    match action {
        "call_tool" => {
            let tool = value
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'tool' field")?
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
                .ok_or("Missing 'response' field")?
                .to_string();

            Ok(AgentAction::FinalResponse(content))
        }

        _ => Err(format!("Unknown action: {}", action)),
    }
}
