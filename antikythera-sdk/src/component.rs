//! WASM Component Implementation with Host Imports
//!
//! This module implements the WASM Component Model interface.
//! The agent delegates I/O operations (LLM API, tool execution) to the host via imports.

// Auto-generated bindings from WIT will be placed here by cargo-component
// For now, we define the interface structure manually

use serde::{Deserialize, Serialize};

// ============================================================================
// Host Import Types (from WIT)
// ============================================================================

/// LLM request from agent to host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub system_prompt: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub response_format: Option<String>,
}

/// LLM response from host to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
}

/// Tool call event from agent to host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    pub tool_name: String,
    pub arguments_json: String,
    pub session_id: Option<String>,
    pub step_id: u32,
}

/// Tool execution result from host to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub success: bool,
    pub output_json: String,
    pub error_message: Option<String>,
    pub step_id: u32,
}

/// Logging event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
    pub timestamp: Option<String>,
}

// ============================================================================
// Host Trait (what WASM expects host to implement)
// ============================================================================

/// Trait representing host imports
/// The host language (TypeScript/Python/etc) must implement this
#[async_trait::async_trait]
pub trait HostImports {
    /// Call LLM API (host implements actual API call)
    async fn call_llm(&self, request: LlmRequest) -> Result<LlmResponse, String>;
    
    /// Emit tool call event to host for execution
    async fn emit_tool_call(&self, event: ToolCallEvent) -> Result<ToolExecutionResult, String>;
    
    /// Log message to host
    fn log_message(&self, event: LogEvent);
    
    /// Save agent state to host storage
    async fn save_state(&self, context_id: String, state_json: String) -> Result<(), String>;
    
    /// Load agent state from host storage
    async fn load_state(&self, context_id: String) -> Result<Option<String>, String>;
}

// ============================================================================
// Agent Runner with Host Delegation
// ============================================================================

/// Agent that delegates I/O to host via imports
pub struct DelegatingAgent<H: HostImports> {
    host: H,
    session_id: Option<String>,
    step_counter: u32,
    max_steps: u32,
}

impl<H: HostImports> DelegatingAgent<H> {
    /// Create new delegating agent
    pub fn new(host: H, session_id: Option<String>, max_steps: u32) -> Self {
        Self {
            host,
            session_id,
            step_counter: 0,
            max_steps,
        }
    }
    
    /// Run agent with prompt
    pub async fn run(&mut self, prompt: String, system_prompt: String) -> Result<String, String> {
        self.log("info", format!("Starting agent run with prompt: {}", &prompt[..prompt.len().min(50)]));
        
        let mut conversation_history = Vec::new();
        let mut current_prompt = prompt.clone();
        
        loop {
            if self.step_counter >= self.max_steps {
                return Err("Max steps exceeded".to_string());
            }
            
            // Call LLM via host
            let llm_request = LlmRequest {
                prompt: current_prompt.clone(),
                system_prompt: system_prompt.clone(),
                temperature: Some(0.7),
                max_tokens: Some(4096),
                response_format: Some("json_object".to_string()),
            };
            
            self.log("debug", "Calling LLM via host");
            let llm_response = self.host.call_llm(llm_request).await?;
            
            self.log("debug", format!("LLM responded with {} tokens", 
                llm_response.tokens_used.unwrap_or(0)));
            
            // Parse LLM response
            let response_json: serde_json::Value = serde_json::from_str(&llm_response.content)
                .map_err(|e| format!("Failed to parse LLM response as JSON: {}", e))?;
            
            // Check if it's a tool call or final response
            if let Some(action) = response_json.get("action").and_then(|v| v.as_str()) {
                match action {
                    "call_tool" | "call_tools" => {
                        self.step_counter += 1;
                        
                        // Extract tool call details
                        let tool_name = response_json.get("tool")
                            .or_else(|| response_json.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let tool_args = response_json.get("input")
                            .or_else(|| response_json.get("arguments"))
                            .cloned()
                            .unwrap_or(serde_json::json!({}));
                        
                        // Emit tool call to host
                        let event = ToolCallEvent {
                            tool_name: tool_name.clone(),
                            arguments_json: tool_args.to_string(),
                            session_id: self.session_id.clone(),
                            step_id: self.step_counter,
                        };
                        
                        self.log("info", format!("Emitting tool call: {}", tool_name));
                        let tool_result = self.host.emit_tool_call(event).await?;
                        
                        // Prepare tool result prompt for next LLM call
                        current_prompt = format!(
                            "Tool '{}' executed. Result: {}\n\nContinue with the task.",
                            tool_result.tool_name,
                            tool_result.output_json
                        );
                        
                        conversation_history.push(serde_json::json!({
                            "step": self.step_counter,
                            "action": "tool_call",
                            "tool": tool_name,
                            "result": tool_result.success,
                        }));
                    }
                    "final" => {
                        // Final response from AI
                        let final_content = response_json.get("response")
                            .or_else(|| response_json.get("content"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("No content")
                            .to_string();
                        
                        self.log("info", "Agent completed with final response");
                        
                        // Save final state
                        let state = serde_json::json!({
                            "status": "completed",
                            "response": final_content,
                            "steps": self.step_counter,
                            "session_id": self.session_id,
                        });
                        
                        if let Some(session_id) = &self.session_id {
                            let _ = self.host.save_state(
                                session_id.clone(),
                                state.to_string()
                            ).await;
                        }
                        
                        return Ok(final_content);
                    }
                    _ => {
                        // Unknown action - retry with clarification
                        current_prompt = format!(
                            "Unknown action: '{}'. Please respond with either a tool call or final response.",
                            action
                        );
                    }
                }
            } else {
                // No action field - assume it's a text response
                self.log("warn", "LLM response missing 'action' field");
                current_prompt = "Please respond in JSON format with either an 'action' field.".to_string();
            }
        }
    }
    
    /// Helper to log messages
    fn log(&self, level: &str, message: String) {
        let event = LogEvent {
            level: level.to_string(),
            message,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        self.host.log_message(event);
    }
}

// ============================================================================
// Export Functions for FFI
// ============================================================================

/// Run agent with host delegation (FFI entry point)
#[no_mangle]
pub extern "C" fn run_agent_with_host(
    prompt_ptr: *const std::os::raw::c_char,
    system_prompt_ptr: *const std::os::raw::c_char,
    session_id_ptr: *const std::os::raw::c_char,
    max_steps: u32,
) -> *mut std::os::raw::c_char {
    use std::ffi::{CStr, CString};
    
    let prompt = unsafe {
        CStr::from_ptr(prompt_ptr)
            .to_str()
            .unwrap_or("")
            .to_string()
    };
    
    let system_prompt = unsafe {
        CStr::from_ptr(system_prompt_ptr)
            .to_str()
            .unwrap_or("")
            .to_string()
    };
    
    let session_id = if session_id_ptr.is_null() {
        None
    } else {
        unsafe {
            CStr::from_ptr(session_id_ptr)
                .to_str()
                .ok()
                .map(String::from)
        }
    };
    
    // Note: This is a placeholder - actual implementation would need
    // a concrete HostImports implementation from the host language
    let result = format!(
        "Agent would run with prompt: {}, system: {}, session: {:?}, max_steps: {}",
        prompt, system_prompt, session_id, max_steps
    );
    
    CString::new(result).unwrap().into_raw()
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ========================================================================
    // Mock Host Implementation for Testing
    // ========================================================================

    struct MockHostConfig {
        pub llm_responses: Vec<String>,
        pub tool_outputs: Vec<String>,
        pub tool_errors: Vec<Option<String>>,
    }

    struct MockHost {
        config: Arc<Mutex<MockHostConfig>>,
        llm_call_count: Arc<Mutex<u32>>,
        tool_call_count: Arc<Mutex<u32>>,
        logs: Arc<Mutex<Vec<LogEvent>>>,
        saved_states: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl MockHost {
        fn new(config: MockHostConfig) -> Self {
            Self {
                config: Arc::new(Mutex::new(config)),
                llm_call_count: Arc::new(Mutex::new(0)),
                tool_call_count: Arc::new(Mutex::new(0)),
                logs: Arc::new(Mutex::new(Vec::new())),
                saved_states: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl HostImports for MockHost {
        async fn call_llm(&self, request: LlmRequest) -> Result<LlmResponse, String> {
            let mut count = self.llm_call_count.lock().unwrap();
            *count += 1;
            let idx = (*count - 1) as usize;

            let config = self.config.lock().unwrap();
            if idx < config.llm_responses.len() {
                Ok(LlmResponse {
                    content: config.llm_responses[idx].clone(),
                    model: "mock-model".to_string(),
                    tokens_used: Some(100),
                    finish_reason: Some("stop".to_string()),
                })
            } else {
                Err("No more mock LLM responses".to_string())
            }
        }

        async fn emit_tool_call(&self, event: ToolCallEvent) -> Result<ToolExecutionResult, String> {
            let mut count = self.tool_call_count.lock().unwrap();
            *count += 1;
            let idx = (*count - 1) as usize;

            let config = self.config.lock().unwrap();
            let success = config.tool_errors.get(idx).map_or(true, |e| e.is_none());
            let error_msg = config.tool_errors.get(idx).and_then(|e| e.clone());
            let output = config.tool_outputs.get(idx)
                .cloned()
                .unwrap_or_else(|| format!("{{\"tool\": \"{}\"}}", event.tool_name));

            Ok(ToolExecutionResult {
                tool_name: event.tool_name.clone(),
                success,
                output_json: output,
                error_message: error_msg,
                step_id: event.step_id,
            })
        }

        fn log_message(&self, event: LogEvent) {
            let mut logs = self.logs.lock().unwrap();
            logs.push(event);
        }

        async fn save_state(&self, context_id: String, state_json: String) -> Result<(), String> {
            let mut states = self.saved_states.lock().unwrap();
            states.push((context_id, state_json));
            Ok(())
        }

        async fn load_state(&self, context_id: String) -> Result<Option<String>, String> {
            let states = self.saved_states.lock().unwrap();
            Ok(states.iter()
                .find(|(id, _)| id == &context_id)
                .map(|(_, state)| state.clone()))
        }
    }

    // Helper to create agent
    fn create_test_agent(host: MockHost, session_id: Option<String>, max_steps: u32) -> DelegatingAgent<MockHost> {
        DelegatingAgent::new(host, session_id, max_steps)
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_llm_request_serialization() {
        let request = LlmRequest {
            prompt: "Hello".to_string(),
            system_prompt: "You are helpful".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            response_format: Some("json_object".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: LlmRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.prompt, "Hello");
        assert_eq!(deserialized.system_prompt, "You are helpful");
        assert_eq!(deserialized.temperature, Some(0.7));
        assert_eq!(deserialized.max_tokens, Some(1000));
    }

    #[test]
    fn test_llm_response_serialization() {
        let response = LlmResponse {
            content: "Test response".to_string(),
            model: "gpt-4".to_string(),
            tokens_used: Some(150),
            finish_reason: Some("stop".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: LlmResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.content, "Test response");
        assert_eq!(deserialized.model, "gpt-4");
        assert_eq!(deserialized.tokens_used, Some(150));
    }

    #[test]
    fn test_tool_call_event_serialization() {
        let event = ToolCallEvent {
            tool_name: "get_weather".to_string(),
            arguments_json: r#"{"city": "NYC"}"#.to_string(),
            session_id: Some("session-123".to_string()),
            step_id: 1,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ToolCallEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.tool_name, "get_weather");
        assert_eq!(deserialized.arguments_json, r#"{"city": "NYC"}"#);
        assert_eq!(deserialized.session_id, Some("session-123".to_string()));
        assert_eq!(deserialized.step_id, 1);
    }

    #[test]
    fn test_tool_execution_result_serialization() {
        let result = ToolExecutionResult {
            tool_name: "get_weather".to_string(),
            success: true,
            output_json: r#"{"temp": 72}"#.to_string(),
            error_message: None,
            step_id: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ToolExecutionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.tool_name, "get_weather");
        assert!(deserialized.success);
        assert_eq!(deserialized.output_json, r#"{"temp": 72}"#);
        assert!(deserialized.error_message.is_none());
    }

    #[test]
    fn test_log_event_serialization() {
        let event = LogEvent {
            level: "info".to_string(),
            message: "Test log".to_string(),
            timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: LogEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.level, "info");
        assert_eq!(deserialized.message, "Test log");
        assert_eq!(deserialized.timestamp, Some("2024-01-01T00:00:00Z".to_string()));
    }

    // ========================================================================
    // Agent Tests - Direct Response
    // ========================================================================

    #[tokio::test]
    async fn test_agent_direct_final_response() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "response": "Hello! How can I help you?"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, Some("test-session".to_string()), 5);

        let result = agent.run("Hello".to_string(), "You are helpful".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello! How can I help you?");

        // Verify LLM was called once
        assert_eq!(*agent.host.llm_call_count.lock().unwrap(), 1);
        assert_eq!(*agent.host.tool_call_count.lock().unwrap(), 0);

        // Verify state was saved
        assert_eq!(agent.host.saved_states.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_agent_final_response_with_content_field() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "content": "Content field response"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Content field response");
    }

    #[tokio::test]
    async fn test_agent_final_response_no_content_fallback() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "No content");
    }

    // ========================================================================
    // Agent Tests - Tool Call
    // ========================================================================

    #[tokio::test]
    async fn test_agent_single_tool_call() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "tool": "get_weather", "input": {"city": "NYC"}}"#.to_string(),
                r#"{"action": "final", "response": "It's 72°F in NYC"}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{"temp": 72, "unit": "F"}"#.to_string(),
            ],
            tool_errors: vec![None],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, Some("weather-session".to_string()), 5);

        let result = agent.run("What's the weather?".to_string(), "You are helpful".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "It's 72°F in NYC");

        // Verify counts
        assert_eq!(*agent.host.llm_call_count.lock().unwrap(), 2);
        assert_eq!(*agent.host.tool_call_count.lock().unwrap(), 1);

        // Verify logs contain tool call
        let logs = agent.host.logs.lock().unwrap();
        assert!(logs.iter().any(|l| l.message.contains("Emitting tool call: get_weather")));
    }

    #[tokio::test]
    async fn test_agent_tool_call_with_name_field() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "name": "search", "arguments": {"query": "Rust"}}"#.to_string(),
                r#"{"action": "final", "response": "Found results"}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{"results": ["Rust lang"]}"#.to_string(),
            ],
            tool_errors: vec![None],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Search for Rust".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Found results");
        assert_eq!(*agent.host.tool_call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_agent_multiple_tool_calls() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "tool": "get_weather", "input": {"city": "NYC"}}"#.to_string(),
                r#"{"action": "call_tool", "tool": "get_time", "input": {"timezone": "EST"}}"#.to_string(),
                r#"{"action": "final", "response": "Weather: 72°F, Time: 3:00 PM"}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{"temp": 72}"#.to_string(),
                r#"{"time": "15:00"}"#.to_string(),
            ],
            tool_errors: vec![None, None],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, Some("multi-tool".to_string()), 10);

        let result = agent.run("Weather and time in NYC".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Weather: 72°F, Time: 3:00 PM");

        assert_eq!(*agent.host.llm_call_count.lock().unwrap(), 3);
        assert_eq!(*agent.host.tool_call_count.lock().unwrap(), 2);
    }

    // ========================================================================
    // Agent Tests - Error Handling
    // ========================================================================

    #[tokio::test]
    async fn test_agent_max_steps_exceeded() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "tool": "tool1", "input": {}}"#.to_string(),
                r#"{"action": "call_tool", "tool": "tool2", "input": {}}"#.to_string(),
                r#"{"action": "call_tool", "tool": "tool3", "input": {}}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{}"#.to_string(),
                r#"{}"#.to_string(),
                r#"{}"#.to_string(),
            ],
            tool_errors: vec![None, None, None],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 2); // Max 2 steps

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Max steps exceeded");
    }

    #[tokio::test]
    async fn test_agent_invalid_json_response() {
        let config = MockHostConfig {
            llm_responses: vec![
                "This is not JSON".to_string(),
                r#"{"action": "final", "response": "Retry success"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON"));
    }

    #[tokio::test]
    async fn test_agent_unknown_action() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "unknown_action"}"#.to_string(),
                r#"{"action": "final", "response": "After retry"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "After retry");
    }

    #[tokio::test]
    async fn test_agent_missing_action_field() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"response": "No action field"}"#.to_string(),
                r#"{"action": "final", "response": "Fixed"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Fixed");

        // Verify warning was logged
        let logs = agent.host.logs.lock().unwrap();
        assert!(logs.iter().any(|l| l.message.contains("missing 'action'")));
    }

    #[tokio::test]
    async fn test_agent_llm_error() {
        let config = MockHostConfig {
            llm_responses: vec![],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No more mock LLM responses"));
    }

    #[tokio::test]
    async fn test_agent_tool_error() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "tool": "failing_tool", "input": {}}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{"error": true}"#.to_string(),
            ],
            tool_errors: vec![Some("Tool execution failed".to_string())],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("Test".to_string(), "System".to_string()).await;
        assert!(result.is_ok()); // Tool error doesn't fail the agent, it continues

        let tool_result = agent.host.emit_tool_call(ToolCallEvent {
            tool_name: "test".to_string(),
            arguments_json: "{}".to_string(),
            session_id: None,
            step_id: 1,
        }).await;

        assert!(tool_result.is_ok());
        assert!(!tool_result.unwrap().success);
    }

    // ========================================================================
    // State Management Tests
    // ========================================================================

    #[tokio::test]
    async fn test_save_and_load_state() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "response": "Done"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let session_id = "session-123".to_string();

        // Save state manually
        let state = r#"{"step": 1, "status": "running"}"#;
        host.save_state(session_id.clone(), state.to_string()).await.unwrap();

        // Load state
        let loaded = host.load_state(session_id.clone()).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), state);
    }

    #[tokio::test]
    async fn test_load_nonexistent_state() {
        let config = MockHostConfig {
            llm_responses: vec![],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);

        let loaded = host.load_state("nonexistent".to_string()).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_agent_saves_final_state() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "response": "Final answer"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let session_id = "test-state".to_string();
        let mut agent = create_test_agent(host, Some(session_id.clone()), 5);

        let _ = agent.run("Test".to_string(), "System".to_string()).await;

        // Verify state was saved
        let states = agent.host.saved_states.lock().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].0, session_id);

        let saved_state: serde_json::Value = serde_json::from_str(&states[0].1).unwrap();
        assert_eq!(saved_state["status"], "completed");
        assert_eq!(saved_state["response"], "Final answer");
        assert_eq!(saved_state["steps"], 0);
    }

    // ========================================================================
    // Logging Tests
    // ========================================================================

    #[tokio::test]
    async fn test_agent_logs_execution() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "response": "Hi"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let _ = agent.run("Test".to_string(), "System".to_string()).await;

        let logs = agent.host.logs.lock().unwrap();
        assert!(!logs.is_empty());
        assert!(logs.iter().any(|l| l.level == "info" && l.message.contains("Starting agent run")));
        assert!(logs.iter().any(|l| l.level == "debug" && l.message.contains("Calling LLM")));
        assert!(logs.iter().any(|l| l.level == "info" && l.message.contains("completed")));
    }

    // ========================================================================
    // FFI Function Tests
    // ========================================================================

    #[test]
    fn test_run_agent_with_host_ffi() {
        use std::ffi::{CStr, CString};

        let prompt = CString::new("Test prompt").unwrap();
        let system_prompt = CString::new("Test system").unwrap();
        let session_id = CString::new("session-1").unwrap();

        let result_ptr = run_agent_with_host(
            prompt.as_ptr(),
            system_prompt.as_ptr(),
            session_id.as_ptr(),
            5,
        );

        let result = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap().to_string() };
        assert!(result.contains("Test prompt"));
        assert!(result.contains("Test system"));
        assert!(result.contains("session-1"));
        assert!(result.contains("max_steps: 5"));

        // Clean up
        unsafe { drop(CString::from_raw(result_ptr)) };
    }

    #[test]
    fn test_run_agent_with_host_ffi_null_session() {
        use std::ffi::{CStr, CString};

        let prompt = CString::new("Prompt only").unwrap();
        let system_prompt = CString::new("System only").unwrap();

        let result_ptr = run_agent_with_host(
            prompt.as_ptr(),
            system_prompt.as_ptr(),
            std::ptr::null(),
            10,
        );

        let result = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap().to_string() };
        assert!(result.contains("Prompt only"));
        assert!(result.contains("System only"));
        assert!(result.contains("session: None"));
        assert!(result.contains("max_steps: 10"));

        unsafe { drop(CString::from_raw(result_ptr)) };
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[tokio::test]
    async fn test_agent_empty_prompt() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "final", "response": "Empty response"}"#.to_string(),
            ],
            tool_outputs: vec![],
            tool_errors: vec![],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Empty_response");
    }

    #[tokio::test]
    async fn test_agent_tool_call_without_args() {
        let config = MockHostConfig {
            llm_responses: vec![
                r#"{"action": "call_tool", "tool": "list_files"}"#.to_string(),
                r#"{"action": "final", "response": "Done"}"#.to_string(),
            ],
            tool_outputs: vec![
                r#"{"files": []}"#.to_string(),
            ],
            tool_errors: vec![None],
        };

        let host = MockHost::new(config);
        let mut agent = create_test_agent(host, None, 5);

        let result = agent.run("List files".to_string(), "System".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(*agent.host.tool_call_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_all_types_are_serializable() {
        // Ensure all types can be serialized and deserialized
        let llm_req = LlmRequest {
            prompt: "test".to_string(),
            system_prompt: "test".to_string(),
            temperature: Some(0.5),
            max_tokens: Some(100),
            response_format: Some("json".to_string()),
        };
        assert!(serde_json::to_string(&llm_req).is_ok());

        let llm_res = LlmResponse {
            content: "test".to_string(),
            model: "test".to_string(),
            tokens_used: Some(50),
            finish_reason: Some("stop".to_string()),
        };
        assert!(serde_json::to_string(&llm_res).is_ok());

        let tool_evt = ToolCallEvent {
            tool_name: "test".to_string(),
            arguments_json: "{}".to_string(),
            session_id: None,
            step_id: 0,
        };
        assert!(serde_json::to_string(&tool_evt).is_ok());

        let tool_res = ToolExecutionResult {
            tool_name: "test".to_string(),
            success: true,
            output_json: "{}".to_string(),
            error_message: None,
            step_id: 0,
        };
        assert!(serde_json::to_string(&tool_res).is_ok());

        let log = LogEvent {
            level: "info".to_string(),
            message: "test".to_string(),
            timestamp: None,
        };
        assert!(serde_json::to_string(&log).is_ok());
    }
}
