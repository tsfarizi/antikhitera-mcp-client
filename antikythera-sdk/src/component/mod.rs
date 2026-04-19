//! WASM Component Feature Slice
//!
//! Provides WASM Component Model interface with host imports for I/O delegation.

use serde::{Deserialize, Serialize};

// ============================================================================
// Host Import Types
// ============================================================================

/// LLM request from agent to host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub messages_json: String,
    pub force_json: bool,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub schema_name: Option<String>,
    pub metadata_json: Option<String>,
}

/// LLM response from host to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub message_json: Option<String>,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
    pub raw_response_json: Option<String>,
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
// Host Trait
// ============================================================================

/// Trait representing host imports
#[async_trait::async_trait]
pub trait HostImports {
    async fn call_llm(&self, request: LlmRequest) -> Result<LlmResponse, String>;
    async fn emit_tool_call(&self, event: ToolCallEvent) -> Result<ToolExecutionResult, String>;
    fn log_message(&self, event: LogEvent);
    async fn save_state(&self, context_id: String, state_json: String) -> Result<(), String>;
    async fn load_state(&self, context_id: String) -> Result<Option<String>, String>;
}

// ============================================================================
// Delegating Agent
// ============================================================================

pub struct DelegatingAgent<H: HostImports> {
    host: H,
    session_id: Option<String>,
    step_counter: u32,
    max_steps: u32,
}

impl<H: HostImports> DelegatingAgent<H> {
    pub fn new(host: H, session_id: Option<String>, max_steps: u32) -> Self {
        Self {
            host,
            session_id,
            step_counter: 0,
            max_steps,
        }
    }

    pub async fn run(&mut self, prompt: String, system_prompt: String) -> Result<String, String> {
        self.log(
            "info",
            format!("Starting agent run: {}", &prompt[..prompt.len().min(50)]),
        );

        let mut current_prompt = prompt.clone();

        loop {
            if self.step_counter >= self.max_steps {
                return Err("Max steps exceeded".to_string());
            }

            let messages_json = serde_json::json!([
                {
                    "role": "system",
                    "content": system_prompt,
                },
                {
                    "role": "user",
                    "content": current_prompt,
                }
            ])
            .to_string();

            let llm_request = LlmRequest {
                provider: None,
                model: None,
                session_id: self.session_id.clone(),
                messages_json,
                force_json: true,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                schema_name: None,
                metadata_json: None,
            };

            let llm_response = self.host.call_llm(llm_request).await?;
            let response_body = llm_response
                .message_json
                .clone()
                .or(llm_response.raw_response_json.clone())
                .unwrap_or_else(|| llm_response.content.clone());

            let response_json: serde_json::Value = match serde_json::from_str(&response_body) {
                Ok(value) => value,
                Err(_) => {
                    self.log(
                        "info",
                        "Host returned plain text response; finishing session".to_string(),
                    );
                    return Ok(llm_response.content);
                }
            };

            if let Some(action) = response_json.get("action").and_then(|v| v.as_str()) {
                match action {
                    "call_tool" | "call_tools" => {
                        self.step_counter += 1;
                        let tool_name = response_json
                            .get("tool")
                            .or_else(|| response_json.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let tool_args = response_json
                            .get("input")
                            .or_else(|| response_json.get("arguments"))
                            .cloned()
                            .unwrap_or(serde_json::json!({}));

                        let event = ToolCallEvent {
                            tool_name: tool_name.clone(),
                            arguments_json: tool_args.to_string(),
                            session_id: self.session_id.clone(),
                            step_id: self.step_counter,
                        };

                        self.log("info", format!("Tool call: {}", tool_name));
                        let tool_result = self.host.emit_tool_call(event).await?;

                        current_prompt = format!(
                            "Tool '{}' executed. Result: {}\n\nContinue.",
                            tool_result.tool_name, tool_result.output_json
                        );
                    }
                    "final" => {
                        let final_content = response_json
                            .get("response")
                            .or_else(|| response_json.get("content"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("No content")
                            .to_string();

                        self.log("info", "Agent completed".to_string());

                        if let Some(session_id) = &self.session_id {
                            let state = serde_json::json!({
                                "status": "completed",
                                "response": final_content,
                                "steps": self.step_counter,
                            });
                            let _ = self
                                .host
                                .save_state(session_id.clone(), state.to_string())
                                .await;
                        }

                        return Ok(final_content);
                    }
                    _ => {
                        current_prompt =
                            "Unknown action. Please respond with tool call or final response."
                                .to_string();
                    }
                }
            } else {
                self.log("warn", "LLM response missing 'action' field".to_string());
                current_prompt = "Please respond in JSON format.".to_string();
            }
        }
    }

    fn log(&self, level: &str, message: String) {
        let event = LogEvent {
            level: level.to_string(),
            message,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        self.host.log_message(event);
    }
}

/// FFI entry point for running agent with host delegation
///
/// # Safety
/// The caller must ensure all non-null pointers are valid NUL-terminated C strings
/// for the duration of this call.
pub unsafe fn run_agent_with_host(
    prompt_ptr: *const std::os::raw::c_char,
    system_prompt_ptr: *const std::os::raw::c_char,
    session_id_ptr: *const std::os::raw::c_char,
    max_steps: u32,
) -> *mut std::os::raw::c_char {
    use std::ffi::{CStr, CString};

    let prompt = unsafe { CStr::from_ptr(prompt_ptr) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let system_prompt = unsafe { CStr::from_ptr(system_prompt_ptr) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let session_id = if session_id_ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(session_id_ptr) }
            .to_str()
            .ok()
            .map(String::from)
    };

    let result = format!(
        "Agent would run with prompt: {}, system: {}, session: {:?}, max_steps: {}",
        prompt, system_prompt, session_id, max_steps
    );

    CString::new(result).unwrap().into_raw()
}
