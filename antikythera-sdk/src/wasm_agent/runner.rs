use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use super::processor::{build_llm_messages, process_llm_response, process_tool_result};
use super::types::{
    AgentAction, AgentConfig, AgentMessage, AgentState, ToolCall, ToolResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunnerConfigInput {
    pub max_steps: Option<u32>,
    pub verbose: Option<bool>,
    pub auto_execute_tools: Option<bool>,
    pub session_timeout_secs: Option<u32>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrepareUserTurnInput {
    pub prompt: String,
    pub session_id: Option<String>,
    pub system_prompt: Option<String>,
    pub force_json: Option<bool>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreparedTurn {
    pub session_id: String,
    pub step: u32,
    pub prompt: String,
    pub system_prompt: String,
    pub force_json: bool,
    pub metadata_json: Option<String>,
    pub messages_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitResult {
    pub session_id: String,
    pub step: u32,
    pub action: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolResultInput {
    pub tool_name: String,
    pub success: bool,
    pub output_json: String,
    pub error_message: Option<String>,
}

#[derive(Default)]
struct AgentRunnerRuntime {
    sessions: HashMap<String, AgentState>,
    default_config: AgentConfig,
}

impl AgentRunnerRuntime {
    fn ensure_session(&mut self, session_id: &str) -> &mut AgentState {
        self.sessions.entry(session_id.to_string()).or_insert_with(|| {
            let mut config = self.default_config.clone();
            config.session_id = session_id.to_string();
            AgentState::new(config)
        })
    }

    fn configure(&mut self, config_json: &str) -> Result<String, String> {
        let input: RunnerConfigInput = serde_json::from_str(config_json)
            .map_err(|e| format!("Invalid config-json: {e}"))?;

        if let Some(value) = input.max_steps {
            self.default_config.max_steps = value;
        }
        if let Some(value) = input.verbose {
            self.default_config.verbose = value;
        }
        if let Some(value) = input.auto_execute_tools {
            self.default_config.auto_execute_tools = value;
        }
        if let Some(value) = input.session_timeout_secs {
            self.default_config.session_timeout_secs = value;
        }

        let session_id = input.session_id.unwrap_or_else(new_session_id);
        let mut config = self.default_config.clone();
        config.session_id = session_id.clone();
        self.sessions
            .entry(session_id.clone())
            .or_insert_with(|| AgentState::new(config));

        Ok(session_id)
    }

    fn prepare_user_turn(&mut self, request_json: &str) -> Result<String, String> {
        let input: PrepareUserTurnInput = serde_json::from_str(request_json)
            .map_err(|e| format!("Invalid request-json: {e}"))?;

        let session_id = input.session_id.unwrap_or_else(new_session_id);
        let state = self.ensure_session(&session_id);
        let system_prompt = input.system_prompt.unwrap_or_default();
        let mut messages = build_llm_messages(&system_prompt, state);
        messages.push(HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), input.prompt.clone()),
        ]));

        let prepared = PreparedTurn {
            session_id,
            step: state.current_step,
            prompt: input.prompt,
            system_prompt,
            force_json: input.force_json.unwrap_or(false),
            metadata_json: input.metadata_json,
            messages_json: serde_json::to_string(&messages)
                .map_err(|e| format!("Failed to encode messages_json: {e}"))?,
        };

        serde_json::to_string(&prepared).map_err(|e| format!("Failed to encode prepared turn: {e}"))
    }

    fn commit_llm_response(
        &mut self,
        prepared_turn_json: &str,
        llm_response_json: &str,
    ) -> Result<String, String> {
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json)
            .map_err(|e| format!("Invalid prepared-turn-json: {e}"))?;

        let state = self.ensure_session(&prepared.session_id);
        state.add_message(AgentMessage {
            role: "user".to_string(),
            content: prepared.prompt,
            tool_call: None,
            tool_result: None,
        });

        let action = process_llm_response(state, llm_response_json)?;

        let result = match action {
            AgentAction::Final { response } => {
                let content = if let Some(text) = response.as_str() {
                    text.to_string()
                } else {
                    response.to_string()
                };

                state.add_message(AgentMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                    tool_call: None,
                    tool_result: None,
                });

                CommitResult {
                    session_id: state.session_id.clone(),
                    step: state.current_step,
                    action: "final".to_string(),
                    content: Some(content),
                    tool_name: None,
                    tool_input: None,
                }
            }
            AgentAction::CallTool { tool, input } => {
                state.add_message(AgentMessage {
                    role: "assistant".to_string(),
                    content: format!("call_tool:{}", tool),
                    tool_call: Some(ToolCall {
                        name: tool.clone(),
                        arguments: input.clone(),
                        step_id: state.current_step,
                    }),
                    tool_result: None,
                });

                CommitResult {
                    session_id: state.session_id.clone(),
                    step: state.current_step,
                    action: "call_tool".to_string(),
                    content: None,
                    tool_name: Some(tool),
                    tool_input: Some(input),
                }
            }
            AgentAction::Retry { error } => CommitResult {
                session_id: state.session_id.clone(),
                step: state.current_step,
                action: "retry".to_string(),
                content: Some(error),
                tool_name: None,
                tool_input: None,
            },
        };

        serde_json::to_string(&result).map_err(|e| format!("Failed to encode commit result: {e}"))
    }

    fn process_llm_response(&mut self, session_id: &str, llm_response_json: &str) -> Result<String, String> {
        let state = self.ensure_session(session_id);
        let action = process_llm_response(state, llm_response_json)?;
        serde_json::to_string(&action).map_err(|e| format!("Failed to encode action: {e}"))
    }

    fn process_tool_result(&mut self, session_id: &str, tool_result_json: &str) -> Result<String, String> {
        let input: ToolResultInput = serde_json::from_str(tool_result_json)
            .map_err(|e| format!("Invalid tool-result-json: {e}"))?;

        let output: serde_json::Value = serde_json::from_str(&input.output_json)
            .map_err(|e| format!("Invalid tool output_json: {e}"))?;

        let state = self.ensure_session(session_id);
        let next_message = process_tool_result(
            state,
            &input.tool_name,
            input.success,
            output.clone(),
            input.error_message.clone(),
        )?;

        let result = ToolResult {
            name: input.tool_name,
            success: input.success,
            output,
            error: input.error_message,
            step_id: state.current_step,
        };

        serde_json::to_string(&serde_json::json!({
            "session_id": state.session_id,
            "step": state.current_step,
            "next_message": next_message,
            "tool_result": result,
        }))
        .map_err(|e| format!("Failed to encode tool processing result: {e}"))
    }
}

fn runtime() -> &'static Mutex<AgentRunnerRuntime> {
    static RUNTIME: OnceLock<Mutex<AgentRunnerRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(AgentRunnerRuntime::default()))
}

fn with_runtime<T>(f: impl FnOnce(&mut AgentRunnerRuntime) -> Result<T, String>) -> Result<T, String> {
    let mut guard = runtime()
        .lock()
        .map_err(|_| "AgentRunner runtime lock poisoned".to_string())?;
    f(&mut guard)
}

fn new_session_id() -> String {
    format!("session-{}", chrono::Utc::now().timestamp_millis())
}

pub fn init(config_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.configure(config_json))
}

pub fn prepare_user_turn(request_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.prepare_user_turn(request_json))
}

pub fn commit_llm_response(prepared_turn_json: &str, llm_response_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.commit_llm_response(prepared_turn_json, llm_response_json))
}

pub fn process_llm_response_for_session(session_id: &str, llm_response_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.process_llm_response(session_id, llm_response_json))
}

pub fn process_tool_result_for_session(session_id: &str, tool_result_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.process_tool_result(session_id, tool_result_json))
}

pub fn get_state(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| {
        let state = rt
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        state.to_json()
    })
}

pub fn reset_session(session_id: &str) -> Result<bool, String> {
    with_runtime(|rt| Ok(rt.sessions.remove(session_id).is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_and_commit_plain_text_response() {
        let session_id = init(r#"{"max_steps":10}"#).unwrap();

        let prepared = prepare_user_turn(
            &serde_json::json!({
                "prompt": "halo",
                "session_id": session_id,
                "system_prompt": "Anda asisten",
                "force_json": false
            })
            .to_string(),
        )
        .unwrap();

        let committed = commit_llm_response(&prepared, "balasan biasa").unwrap();
        let value: serde_json::Value = serde_json::from_str(&committed).unwrap();
        assert_eq!(value["action"], "final");
        assert_eq!(value["content"], "balasan biasa");
    }

    #[test]
    fn prepare_and_commit_structured_tool_call() {
        let session_id = init(r#"{"max_steps":10}"#).unwrap();

        let prepared = prepare_user_turn(
            &serde_json::json!({
                "prompt": "cek cuaca",
                "session_id": session_id,
                "system_prompt": "Gunakan tool jika perlu",
                "force_json": true
            })
            .to_string(),
        )
        .unwrap();

        let response = serde_json::json!({
            "action": "call_tool",
            "tool": "weather.get",
            "input": {"city": "Jakarta"}
        })
        .to_string();

        let committed = commit_llm_response(&prepared, &response).unwrap();
        let value: serde_json::Value = serde_json::from_str(&committed).unwrap();

        assert_eq!(value["action"], "call_tool");
        assert_eq!(value["tool_name"], "weather.get");
        assert_eq!(value["tool_input"]["city"], "Jakarta");
    }
}
