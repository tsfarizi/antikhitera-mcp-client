use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::processor::{build_llm_messages, process_llm_response, process_tool_result};
use super::types::{
    AgentAction, AgentConfig, AgentMessage, AgentState, ContextPolicy, ContextSummary, ProviderPolicyKey,
    StreamEvent, StreamEventKind, TelemetryCounters, TelemetrySnapshot, ToolCall, ToolResult,
    TruncationStrategy,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunnerConfigInput {
    pub max_steps: Option<u32>,
    pub verbose: Option<bool>,
    pub auto_execute_tools: Option<bool>,
    pub session_timeout_secs: Option<u32>,
    pub session_id: Option<String>,
    pub context_policy: Option<ContextPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrepareUserTurnInput {
    pub prompt: String,
    pub session_id: Option<String>,
    pub system_prompt: Option<String>,
    pub force_json: Option<bool>,
    pub metadata_json: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub correlation_id: Option<String>,
    pub context_policy: Option<ContextPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreparedTurn {
    pub session_id: String,
    pub step: u32,
    pub prompt: String,
    pub system_prompt: String,
    pub force_json: bool,
    pub metadata_json: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub correlation_id: Option<String>,
    pub summary_handoff: Option<ContextSummary>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContextPolicyUpdateInput {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub policy: ContextPolicy,
}

struct SessionRuntime {
    state: AgentState,
    pending_llm_chunks: Vec<String>,
    events: Vec<StreamEvent>,
    seq: u64,
    telemetry: TelemetrySnapshot,
}

impl SessionRuntime {
    fn new(config: AgentConfig) -> Self {
        let session_id = config.session_id.clone();
        Self {
            state: AgentState::new(config),
            pending_llm_chunks: Vec::new(),
            events: Vec::new(),
            seq: 0,
            telemetry: TelemetrySnapshot {
                session_id,
                correlation_id: None,
                counters: TelemetryCounters::default(),
                total_prepare_latency_ms: 0,
                total_commit_latency_ms: 0,
            },
        }
    }

    fn emit_event(
        &mut self,
        kind: StreamEventKind,
        correlation_id: Option<String>,
        payload: serde_json::Value,
    ) {
        self.seq += 1;
        if correlation_id.is_some() {
            self.telemetry.correlation_id = correlation_id.clone();
        }
        self.events.push(StreamEvent {
            seq: self.seq,
            session_id: self.state.session_id.clone(),
            step: self.state.current_step,
            correlation_id,
            kind,
            payload,
        });
    }
}

#[derive(Default)]
struct AgentRunnerRuntime {
    sessions: HashMap<String, SessionRuntime>,
    default_config: AgentConfig,
    policy_overrides: HashMap<String, ContextPolicy>,
}

impl AgentRunnerRuntime {
    fn ensure_session(&mut self, session_id: &str) -> &mut SessionRuntime {
        self.sessions.entry(session_id.to_string()).or_insert_with(|| {
            let mut config = self.default_config.clone();
            config.session_id = session_id.to_string();
            SessionRuntime::new(config)
        })
    }

    fn resolve_policy(&self, request: &PrepareUserTurnInput) -> ContextPolicy {
        if let Some(policy) = &request.context_policy {
            return policy.clone();
        }

        let key = ProviderPolicyKey {
            provider: request.provider.clone(),
            model: request.model.clone(),
        }
        .as_map_key();

        if let Some(key) = key {
            if let Some(policy) = self.policy_overrides.get(&key) {
                return policy.clone();
            }
        }

        self.default_config.context_policy.clone()
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
        if let Some(policy) = input.context_policy {
            self.default_config.context_policy = policy;
        }

        let session_id = input.session_id.unwrap_or_else(new_session_id);
        let mut config = self.default_config.clone();
        config.session_id = session_id.clone();
        self.sessions
            .entry(session_id.clone())
            .or_insert_with(|| SessionRuntime::new(config));

        Ok(session_id)
    }

    fn set_context_policy(&mut self, policy_json: &str) -> Result<bool, String> {
        let input: ContextPolicyUpdateInput = serde_json::from_str(policy_json)
            .map_err(|e| format!("Invalid context-policy-json: {e}"))?;

        if let Some(key) = (ProviderPolicyKey {
            provider: input.provider,
            model: input.model,
        })
        .as_map_key()
        {
            self.policy_overrides.insert(key, input.policy);
            return Ok(true);
        }

        self.default_config.context_policy = input.policy;
        Ok(true)
    }

    fn maybe_update_summary(state: &mut AgentState, policy: &ContextPolicy) -> Option<ContextSummary> {
        if state.message_history.len() <= policy.summarize_after_messages {
            return None;
        }

        let retain = policy.max_history_messages.max(1);
        let total = state.message_history.len();
        let summarize_until = total.saturating_sub(retain);
        let to_summarize = &state.message_history[..summarize_until];

        if to_summarize.is_empty() {
            return None;
        }

        let mut text = to_summarize
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join(" | ");

        let max_chars = policy.summary_max_chars.max(120);
        if text.len() > max_chars {
            text.truncate(max_chars);
            text.push_str("...");
        }

        let next_version = state
            .rolling_summary
            .as_ref()
            .map(|s| s.version + 1)
            .unwrap_or(1);

        let summary = ContextSummary {
            version: next_version,
            text,
            source_messages: to_summarize.len(),
        };

        match policy.truncation_strategy {
            TruncationStrategy::KeepNewest => {
                state.message_history = state
                    .message_history
                    .iter()
                    .skip(summarize_until)
                    .cloned()
                    .collect();
            }
            TruncationStrategy::KeepBalanced => {
                let keep_head = retain / 3;
                let keep_tail = retain.saturating_sub(keep_head);
                let head_iter = state.message_history.iter().take(keep_head).cloned();
                let tail_iter = state
                    .message_history
                    .iter()
                    .skip(total.saturating_sub(keep_tail))
                    .cloned();
                state.message_history = head_iter.chain(tail_iter).collect();
            }
        }

        state.rolling_summary = Some(summary.clone());
        Some(summary)
    }

    fn prepare_user_turn(&mut self, request_json: &str) -> Result<String, String> {
        let started = Instant::now();
        let input: PrepareUserTurnInput = serde_json::from_str(request_json)
            .map_err(|e| format!("Invalid request-json: {e}"))?;

        let session_id = input.session_id.clone().unwrap_or_else(new_session_id);
        let policy = self.resolve_policy(&input);
        let runtime = self.ensure_session(&session_id);

        let summary = Self::maybe_update_summary(&mut runtime.state, &policy);
        if let Some(summary) = &summary {
            runtime.telemetry.counters.context_summaries += 1;
            runtime.emit_event(
                StreamEventKind::SummaryUpdated,
                input.correlation_id.clone(),
                serde_json::json!({
                    "version": summary.version,
                    "source_messages": summary.source_messages,
                }),
            );
        }

        let system_prompt = input.system_prompt.clone().unwrap_or_default();
        let mut messages = build_llm_messages(&system_prompt, &runtime.state);
        messages.push(HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), input.prompt.clone()),
        ]));

        runtime.telemetry.counters.turns_prepared += 1;
        runtime.telemetry.total_prepare_latency_ms += started.elapsed().as_millis() as u64;
        runtime.emit_event(
            StreamEventKind::UserTurnPrepared,
            input.correlation_id.clone(),
            serde_json::json!({
                "messages_count": messages.len(),
                "provider": input.provider,
                "model": input.model,
            }),
        );

        let prepared = PreparedTurn {
            session_id,
            step: runtime.state.current_step,
            prompt: input.prompt,
            system_prompt,
            force_json: input.force_json.unwrap_or(false),
            metadata_json: input.metadata_json,
            provider: input.provider,
            model: input.model,
            correlation_id: input.correlation_id,
            summary_handoff: summary.or_else(|| runtime.state.rolling_summary.clone()),
            messages_json: serde_json::to_string(&messages)
                .map_err(|e| format!("Failed to encode messages_json: {e}"))?,
        };

        serde_json::to_string(&prepared).map_err(|e| format!("Failed to encode prepared turn: {e}"))
    }

    fn append_llm_chunk(&mut self, session_id: &str, chunk: &str, correlation_id: Option<String>) -> Result<bool, String> {
        let runtime = self.ensure_session(session_id);
        runtime.pending_llm_chunks.push(chunk.to_string());
        runtime.telemetry.counters.llm_chunks += 1;
        runtime.emit_event(
            StreamEventKind::LlmChunk,
            correlation_id,
            serde_json::json!({"chunk": chunk}),
        );
        Ok(true)
    }

    fn commit_llm_response(
        &mut self,
        prepared_turn_json: &str,
        llm_response_json: &str,
    ) -> Result<String, String> {
        let started = Instant::now();
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json)
            .map_err(|e| format!("Invalid prepared-turn-json: {e}"))?;

        let runtime = self.ensure_session(&prepared.session_id);
        runtime.state.add_message(AgentMessage {
            role: "user".to_string(),
            content: prepared.prompt,
            tool_call: None,
            tool_result: None,
        });

        let action = process_llm_response(&mut runtime.state, llm_response_json)?;
        runtime.telemetry.counters.llm_commits += 1;
        runtime.telemetry.total_commit_latency_ms += started.elapsed().as_millis() as u64;
        runtime.emit_event(
            StreamEventKind::LlmCommitted,
            prepared.correlation_id.clone(),
            serde_json::json!({"length": llm_response_json.len()}),
        );

        let result = match action {
            AgentAction::Final { response } => {
                let content = if let Some(text) = response.as_str() {
                    text.to_string()
                } else {
                    response.to_string()
                };

                runtime.state.add_message(AgentMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                    tool_call: None,
                    tool_result: None,
                });

                runtime.telemetry.counters.final_responses += 1;
                runtime.emit_event(
                    StreamEventKind::FinalResponse,
                    prepared.correlation_id.clone(),
                    serde_json::json!({"content": content}),
                );

                CommitResult {
                    session_id: runtime.state.session_id.clone(),
                    step: runtime.state.current_step,
                    action: "final".to_string(),
                    content: Some(content),
                    tool_name: None,
                    tool_input: None,
                }
            }
            AgentAction::CallTool { tool, input } => {
                runtime.state.add_message(AgentMessage {
                    role: "assistant".to_string(),
                    content: format!("call_tool:{}", tool),
                    tool_call: Some(ToolCall {
                        name: tool.clone(),
                        arguments: input.clone(),
                        step_id: runtime.state.current_step,
                    }),
                    tool_result: None,
                });

                runtime.telemetry.counters.tool_requests += 1;
                runtime.emit_event(
                    StreamEventKind::ToolRequested,
                    prepared.correlation_id.clone(),
                    serde_json::json!({"tool": tool, "input": input}),
                );

                CommitResult {
                    session_id: runtime.state.session_id.clone(),
                    step: runtime.state.current_step,
                    action: "call_tool".to_string(),
                    content: None,
                    tool_name: Some(tool),
                    tool_input: Some(input),
                }
            }
            AgentAction::Retry { error } => CommitResult {
                session_id: runtime.state.session_id.clone(),
                step: runtime.state.current_step,
                action: "retry".to_string(),
                content: Some(error),
                tool_name: None,
                tool_input: None,
            },
        };

        runtime.pending_llm_chunks.clear();
        serde_json::to_string(&result).map_err(|e| format!("Failed to encode commit result: {e}"))
    }

    fn commit_llm_stream(&mut self, prepared_turn_json: &str) -> Result<String, String> {
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json)
            .map_err(|e| format!("Invalid prepared-turn-json: {e}"))?;

        let runtime = self.ensure_session(&prepared.session_id);
        let payload = runtime.pending_llm_chunks.join("");
        self.commit_llm_response(prepared_turn_json, &payload)
    }

    fn process_llm_response(&mut self, session_id: &str, llm_response_json: &str) -> Result<String, String> {
        let runtime = self.ensure_session(session_id);
        let action = process_llm_response(&mut runtime.state, llm_response_json)?;
        serde_json::to_string(&action).map_err(|e| format!("Failed to encode action: {e}"))
    }

    fn process_tool_result(&mut self, session_id: &str, tool_result_json: &str) -> Result<String, String> {
        let input: ToolResultInput = serde_json::from_str(tool_result_json)
            .map_err(|e| format!("Invalid tool-result-json: {e}"))?;

        let output: serde_json::Value = serde_json::from_str(&input.output_json)
            .map_err(|e| format!("Invalid tool output_json: {e}"))?;

        let runtime = self.ensure_session(session_id);
        let next_message = process_tool_result(
            &mut runtime.state,
            &input.tool_name,
            input.success,
            output.clone(),
            input.error_message.clone(),
        )?;

        runtime.telemetry.counters.tool_results += 1;
        runtime.emit_event(
            StreamEventKind::ToolResult,
            runtime.telemetry.correlation_id.clone(),
            serde_json::json!({
                "tool": input.tool_name,
                "success": input.success,
            }),
        );

        let result = ToolResult {
            name: input.tool_name,
            success: input.success,
            output,
            error: input.error_message,
            step_id: runtime.state.current_step,
        };

        serde_json::to_string(&serde_json::json!({
            "session_id": runtime.state.session_id,
            "step": runtime.state.current_step,
            "next_message": next_message,
            "tool_result": result,
        }))
        .map_err(|e| format!("Failed to encode tool processing result: {e}"))
    }

    fn drain_events(&mut self, session_id: &str) -> Result<String, String> {
        let runtime = self.ensure_session(session_id);
        let events = std::mem::take(&mut runtime.events);
        serde_json::to_string(&events).map_err(|e| format!("Failed to encode events: {e}"))
    }

    fn telemetry_snapshot(&mut self, session_id: &str) -> Result<String, String> {
        let runtime = self.ensure_session(session_id);
        runtime.emit_event(
            StreamEventKind::Telemetry,
            runtime.telemetry.correlation_id.clone(),
            serde_json::json!({"snapshot": true}),
        );
        serde_json::to_string(&runtime.telemetry)
            .map_err(|e| format!("Failed to encode telemetry snapshot: {e}"))
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

pub fn set_context_policy(policy_json: &str) -> Result<bool, String> {
    with_runtime(|rt| rt.set_context_policy(policy_json))
}

pub fn prepare_user_turn(request_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.prepare_user_turn(request_json))
}

pub fn append_llm_chunk(session_id: &str, chunk: &str, correlation_id: Option<&str>) -> Result<bool, String> {
    with_runtime(|rt| rt.append_llm_chunk(session_id, chunk, correlation_id.map(|v| v.to_string())))
}

pub fn commit_llm_response(prepared_turn_json: &str, llm_response_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.commit_llm_response(prepared_turn_json, llm_response_json))
}

pub fn commit_llm_stream(prepared_turn_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.commit_llm_stream(prepared_turn_json))
}

pub fn process_llm_response_for_session(session_id: &str, llm_response_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.process_llm_response(session_id, llm_response_json))
}

pub fn process_tool_result_for_session(session_id: &str, tool_result_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.process_tool_result(session_id, tool_result_json))
}

pub fn drain_events(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| rt.drain_events(session_id))
}

pub fn get_telemetry_snapshot(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| rt.telemetry_snapshot(session_id))
}

pub fn get_state(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| {
        let state = rt
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        state.state.to_json()
    })
}

pub fn reset_session(session_id: &str) -> Result<bool, String> {
    with_runtime(|rt| Ok(rt.sessions.remove(session_id).is_some()))
}

