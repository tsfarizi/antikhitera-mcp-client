use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use super::processor::{
    build_llm_messages, process_llm_response, process_tool_result, validate_tool_call,
};
use super::types::{
    AgentAction, AgentConfig, AgentMessage, AgentState, ContextPolicy, StreamEvent,
    StreamEventKind, TelemetryCounters, TelemetrySnapshot, ToolCall, ToolRegistry, ToolResult,
};

use crate::sdk_logging::get_sdk_logger;
use antikythera_log::LogLevel;

mod context_manager;
mod runner_telemetry;
mod runner_types;
use runner_types::*;

pub(super) fn wasm_log(session_id: &str, level: LogLevel, message: &str) {
    get_sdk_logger(session_id).log_with_source(level, "wasm_agent", message);
}

/// Errors raised by the WASM agent runner during session lifecycle operations.
///
/// Returned by all public runner functions to signal failures in session
/// management, LLM interaction, tool execution, and configuration.
#[derive(Debug, Clone)]
pub enum AgentRunnerError {
    SessionNotFound(String),
    SessionArchived(String),
    ValidationFailed(String),
    ToolFailed(String),
    ConfigurationFailed(String),
    Internal(String),
}

impl From<AgentRunnerError> for String {
    fn from(e: AgentRunnerError) -> Self {
        e.to_string()
    }
}

impl std::fmt::Display for AgentRunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound(id) => write!(f, "Session not found: {id}"),
            Self::SessionArchived(id) => write!(f, "Session archived: {id}"),
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {msg}"),
            Self::ToolFailed(msg) => write!(f, "Tool failed: {msg}"),
            Self::ConfigurationFailed(msg) => write!(f, "Configuration failed: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for AgentRunnerError {}

impl From<String> for AgentRunnerError {
    fn from(msg: String) -> Self {
        AgentRunnerError::Internal(msg)
    }
}

struct SessionRuntime {
    state: AgentState,
    pending_llm_chunks: Vec<String>,
    events: Vec<StreamEvent>,
    seq: u64,
    last_touched_ms: i64,
    prepare_latencies_ms: Vec<u64>,
    commit_latencies_ms: Vec<u64>,
    telemetry: TelemetrySnapshot,
}

impl SessionRuntime {
    fn new(config: AgentConfig) -> Self {
        let session_id = config.session_id.clone();
        let now_ms = now_unix_ms();
        Self {
            state: AgentState::new(config),
            pending_llm_chunks: Vec::new(),
            events: Vec::new(),
            seq: 0,
            last_touched_ms: now_ms,
            prepare_latencies_ms: Vec::new(),
            commit_latencies_ms: Vec::new(),
            telemetry: TelemetrySnapshot {
                session_id,
                correlation_id: None,
                counters: TelemetryCounters::default(),
                total_prepare_latency_ms: 0,
                total_commit_latency_ms: 0,
            },
        }
    }

    fn touch(&mut self, now_ms: i64) {
        self.last_touched_ms = now_ms;
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

/// Core WASM agent runtime holding all in-memory sessions, event queues,
/// tool registry, and default configuration for the agent runner.
///
/// Accessed through a global `OnceLock<Mutex<…>>` to provide a singleton
/// runtime for WASM FFI callers.
pub struct AgentRunnerRuntime {
    sessions: HashMap<String, SessionRuntime>,
    archived_sessions: HashMap<String, ArchivedSessionRecord>,
    pending_events: HashMap<String, Vec<StreamEvent>>,
    pending_event_seq: HashMap<String, u64>,
    default_config: AgentConfig,
    max_in_memory_sessions: usize,
    /// Tool definitions pushed from the host (MCP server capabilities).
    known_tools: ToolRegistry,
}

impl Default for AgentRunnerRuntime {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            archived_sessions: HashMap::new(),
            pending_events: HashMap::new(),
            pending_event_seq: HashMap::new(),
            default_config: AgentConfig::default(),
            max_in_memory_sessions: 128,
            known_tools: ToolRegistry::default(),
        }
    }
}

impl AgentRunnerRuntime {
    fn archive_session(
        &mut self,
        session_id: &str,
        reason: &str,
        correlation_id: Option<String>,
    ) -> Result<bool, AgentRunnerError> {
        wasm_log(
            session_id,
            LogLevel::Info,
            &format!("Session archived: {reason}"),
        );
        let Some(runtime) = self.sessions.remove(session_id) else {
            return Ok(false);
        };

        let archived_at_ms = now_unix_ms();
        let state_json = runtime.state.to_json()?;

        self.archived_sessions.insert(
            session_id.to_string(),
            ArchivedSessionRecord {
                archived_at_ms,
                reason: reason.to_string(),
            },
        );

        self.emit_pending_event(
            session_id,
            StreamEventKind::SessionArchived,
            correlation_id,
            serde_json::json!({
                "reason": reason,
                "archived_at_ms": archived_at_ms,
                "last_touched_ms": runtime.last_touched_ms,
                "state_json": state_json,
                "message_count": runtime.state.message_history.len(),
                "step": runtime.state.current_step,
            }),
        );

        Ok(true)
    }

    fn sweep_idle_sessions(&mut self, now_ms: i64) -> Result<u32, AgentRunnerError> {
        let candidates: Vec<String> = self
            .sessions
            .iter()
            .filter_map(|(id, session)| {
                if !session.pending_llm_chunks.is_empty() {
                    return None;
                }
                let timeout_ms = i64::from(session.state.config.session_timeout_secs) * 1_000;
                if timeout_ms <= 0 {
                    return None;
                }
                if now_ms.saturating_sub(session.last_touched_ms) > timeout_ms {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut archived = 0_u32;
        for session_id in candidates {
            if self.archive_session(&session_id, "idle_timeout", None)? {
                archived += 1;
            }
        }
        if archived > 0 {
            wasm_log(
                "runtime",
                LogLevel::Info,
                &format!("Idle sweep archived {archived} sessions"),
            );
        }
        Ok(archived)
    }

    fn enforce_capacity(
        &mut self,
        protected_session_id: Option<&str>,
        correlation_id: Option<String>,
    ) -> Result<u32, AgentRunnerError> {
        if self.max_in_memory_sessions == 0 {
            return Ok(0);
        }

        let mut archived = 0_u32;
        while self.sessions.len() > self.max_in_memory_sessions {
            let candidate = self
                .sessions
                .iter()
                .filter(|(id, session)| {
                    if let Some(protected) = protected_session_id
                        && id.as_str() == protected
                    {
                        return false;
                    }
                    session.pending_llm_chunks.is_empty()
                })
                .min_by_key(|(_, session)| session.last_touched_ms)
                .map(|(id, _)| id.clone());

            let Some(candidate_id) = candidate else {
                break;
            };

            if self.archive_session(&candidate_id, "capacity_pressure", correlation_id.clone())? {
                archived += 1;
            } else {
                break;
            }
        }

        if archived > 0 {
            wasm_log(
                "runtime",
                LogLevel::Info,
                &format!("Capacity pressure archived {archived} sessions"),
            );
        }
        Ok(archived)
    }

    fn ensure_session(&mut self, session_id: &str) -> &mut SessionRuntime {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| {
                let mut config = self.default_config.clone();
                config.session_id = session_id.to_string();
                SessionRuntime::new(config)
            })
    }

    fn resolve_policy(&self, request: &PrepareUserTurnInput) -> ContextPolicy {
        if let Some(policy) = &request.context_policy {
            return policy.clone();
        }
        self.default_config.context_policy.clone()
    }

    fn register_tools(&mut self, tools_json: &str) -> Result<u32, AgentRunnerError> {
        self.known_tools = ToolRegistry::from_json(tools_json)?;
        let count = self.known_tools.len();
        wasm_log(
            "runtime",
            LogLevel::Info,
            &format!("{count} tools registered"),
        );
        Ok(count as u32)
    }

    fn get_tools_prompt(&self) -> Result<String, AgentRunnerError> {
        let block = self.known_tools.to_prompt_block().unwrap_or_default();
        Ok(block)
    }

    fn configure(&mut self, config_json: &str) -> Result<String, AgentRunnerError> {
        let input: RunnerConfigInput = serde_json::from_str(config_json).map_err(|e| {
            AgentRunnerError::ConfigurationFailed(format!("Invalid config-json: {e}"))
        })?;

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
        if let Some(value) = input.max_in_memory_sessions {
            self.max_in_memory_sessions = value.max(1);
        }
        if let Some(policy) = input.context_policy {
            self.default_config.context_policy = policy;
        }

        let session_id = input.session_id.unwrap_or_else(new_session_id);
        let mut config = self.default_config.clone();
        config.session_id = session_id.clone();
        self.sessions.entry(session_id.clone()).or_insert_with(|| {
            wasm_log("runtime", LogLevel::Info, "Session created");
            SessionRuntime::new(config)
        });

        let _ = self.enforce_capacity(Some(&session_id), None)?;

        Ok(session_id)
    }

    fn set_context_policy(&mut self, policy_json: &str) -> Result<bool, AgentRunnerError> {
        let input: ContextPolicyUpdateInput = serde_json::from_str(policy_json).map_err(|e| {
            AgentRunnerError::ConfigurationFailed(format!("Invalid context-policy-json: {e}"))
        })?;
        self.default_config.context_policy = input.policy;
        wasm_log("runtime", LogLevel::Debug, "Context policy updated");
        Ok(true)
    }

    fn prepare_user_turn(&mut self, request_json: &str) -> Result<String, AgentRunnerError> {
        let started = Instant::now();
        let input: PrepareUserTurnInput = serde_json::from_str(request_json).map_err(|e| {
            AgentRunnerError::ValidationFailed(format!("Invalid request-json: {e}"))
        })?;

        let now_ms = now_unix_ms();
        let _ = self.sweep_idle_sessions(now_ms)?;

        // Snapshot the tool block before the mutable session borrow to avoid borrow conflict.
        let tool_block_snapshot = self.known_tools.to_prompt_block();

        let session_id = input.session_id.clone().unwrap_or_else(new_session_id);
        wasm_log(&session_id, LogLevel::Info, "Preparing user turn");

        if !self.sessions.contains_key(&session_id)
            && self.archived_sessions.contains_key(&session_id)
        {
            let archived =
                self.archived_sessions
                    .get(&session_id)
                    .cloned()
                    .unwrap_or(ArchivedSessionRecord {
                        archived_at_ms: now_ms,
                        reason: "unknown".to_string(),
                    });
            self.emit_pending_event(
                &session_id,
                StreamEventKind::SessionRestoreRequested,
                input.correlation_id.clone(),
                serde_json::json!({
                    "reason": archived.reason,
                    "archived_at_ms": archived.archived_at_ms,
                }),
            );
            self.emit_pending_event(
                &session_id,
                StreamEventKind::SessionRestoreProgress,
                input.correlation_id.clone(),
                serde_json::json!({
                    "stage": "requested",
                    "percent": 0,
                    "message": "Host load_state required before this turn can continue"
                }),
            );
            wasm_log(
                &session_id,
                LogLevel::Warn,
                "Session archived, restore required before turn",
            );
            return Err(AgentRunnerError::SessionArchived(format!(
                "Session '{session_id}' archived and not in RAM"
            )));
        }

        let policy = self.resolve_policy(&input);
        let runtime = self.ensure_session(&session_id);
        runtime.touch(now_ms);

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

        let base_system_prompt = input.system_prompt.clone().unwrap_or_default();
        let system_prompt = if let Some(tool_block) = tool_block_snapshot {
            if base_system_prompt.is_empty() {
                tool_block
            } else {
                format!("{base_system_prompt}\n\n{tool_block}")
            }
        } else {
            base_system_prompt
        };
        let mut messages = build_llm_messages(&system_prompt, &runtime.state);
        messages.push(HashMap::from([
            ("role".to_string(), "user".to_string()),
            ("content".to_string(), input.prompt.clone()),
        ]));

        runtime.telemetry.counters.turns_prepared += 1;
        let prepare_latency_ms = started.elapsed().as_millis() as u64;
        runtime.telemetry.total_prepare_latency_ms += prepare_latency_ms;
        runtime.prepare_latencies_ms.push(prepare_latency_ms);
        runtime.emit_event(
            StreamEventKind::UserTurnPrepared,
            input.correlation_id.clone(),
            serde_json::json!({
                "messages_count": messages.len(),
            }),
        );

        let prepared = PreparedTurn {
            session_id,
            step: runtime.state.current_step,
            prompt: input.prompt,
            system_prompt,
            force_json: input.force_json.unwrap_or(false),
            metadata_json: input.metadata_json,
            correlation_id: input.correlation_id,
            summary_handoff: summary.or_else(|| runtime.state.rolling_summary.clone()),
            messages_json: serde_json::to_string(&messages).map_err(|e| {
                AgentRunnerError::Internal(format!("Failed to encode messages_json: {e}"))
            })?,
        };

        let encoded = serde_json::to_string(&prepared).map_err(|e| {
            AgentRunnerError::Internal(format!("Failed to encode prepared turn: {e}"))
        })?;

        let _ =
            self.enforce_capacity(Some(&prepared.session_id), prepared.correlation_id.clone())?;

        Ok(encoded)
    }

    fn append_llm_chunk(
        &mut self,
        session_id: &str,
        chunk: &str,
        correlation_id: Option<String>,
    ) -> Result<bool, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
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
    ) -> Result<String, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let started = Instant::now();
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json).map_err(|e| {
            AgentRunnerError::ValidationFailed(format!("Invalid prepared-turn-json: {e}"))
        })?;

        wasm_log(
            &prepared.session_id,
            LogLevel::Debug,
            "Committing LLM response",
        );

        // Snapshot the registry before the mutable session borrow to avoid borrow conflict.
        let registry_snapshot = self.known_tools.clone();

        let runtime = self.ensure_session(&prepared.session_id);
        runtime.touch(now_unix_ms());
        runtime.state.add_message(AgentMessage {
            role: "user".to_string(),
            content: prepared.prompt,
            tool_call: None,
            tool_result: None,
        });

        let action = process_llm_response(&mut runtime.state, llm_response_json)?;
        runtime.telemetry.counters.llm_commits += 1;
        let commit_latency_ms = started.elapsed().as_millis() as u64;
        runtime.telemetry.total_commit_latency_ms += commit_latency_ms;
        runtime.commit_latencies_ms.push(commit_latency_ms);
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
                // Validate the tool call against the registered registry (no-op if empty).
                if let Err(validation_err) = validate_tool_call(&registry_snapshot, &tool, &input) {
                    wasm_log(
                        &prepared.session_id,
                        LogLevel::Error,
                        &format!("Tool validation failed for '{tool}': {validation_err}"),
                    );
                    return Err(AgentRunnerError::ToolFailed(validation_err.to_string()));
                }

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
            AgentAction::Retry { error } => {
                runtime.telemetry.counters.llm_retries += 1;
                CommitResult {
                    session_id: runtime.state.session_id.clone(),
                    step: runtime.state.current_step,
                    action: "retry".to_string(),
                    content: Some(error),
                    tool_name: None,
                    tool_input: None,
                }
            }
        };

        wasm_log(
            &prepared.session_id,
            LogLevel::Debug,
            &format!("LLM response committed: action={}", result.action),
        );
        runtime.pending_llm_chunks.clear();
        serde_json::to_string(&result)
            .map_err(|e| AgentRunnerError::Internal(format!("Failed to encode commit result: {e}")))
    }

    fn commit_llm_stream(&mut self, prepared_turn_json: &str) -> Result<String, AgentRunnerError> {
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json).map_err(|e| {
            AgentRunnerError::ValidationFailed(format!("Invalid prepared-turn-json: {e}"))
        })?;

        let runtime = self.ensure_session(&prepared.session_id);
        let payload = runtime.pending_llm_chunks.join("");
        self.commit_llm_response(prepared_turn_json, &payload)
    }

    fn process_llm_response(
        &mut self,
        session_id: &str,
        llm_response_json: &str,
    ) -> Result<String, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
        wasm_log(session_id, LogLevel::Debug, "Processing LLM response");
        let action = process_llm_response(&mut runtime.state, llm_response_json)?;
        serde_json::to_string(&action)
            .map_err(|e| AgentRunnerError::Internal(format!("Failed to encode action: {e}")))
    }

    fn process_tool_result(
        &mut self,
        session_id: &str,
        tool_result_json: &str,
    ) -> Result<String, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let input: ToolResultInput = serde_json::from_str(tool_result_json)
            .map_err(|e| AgentRunnerError::ToolFailed(format!("Invalid tool-result-json: {e}")))?;

        wasm_log(
            session_id,
            LogLevel::Debug,
            &format!("Processing tool result for '{}'", input.tool_name),
        );

        let output: serde_json::Value = serde_json::from_str(&input.output_json)
            .map_err(|e| AgentRunnerError::ToolFailed(format!("Invalid tool output_json: {e}")))?;

        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
        let next_message = process_tool_result(
            &mut runtime.state,
            &input.tool_name,
            input.success,
            output.clone(),
            input.error_message.clone(),
        )?;

        runtime.telemetry.counters.tool_results += 1;
        if !input.success {
            runtime.telemetry.counters.tool_errors += 1;
            wasm_log(
                session_id,
                LogLevel::Error,
                &format!(
                    "Tool '{}' failed: {}",
                    input.tool_name,
                    input.error_message.as_deref().unwrap_or("unknown error")
                ),
            );
        }
        runtime.emit_event(
            StreamEventKind::ToolResult,
            input
                .correlation_id
                .clone()
                .or_else(|| runtime.telemetry.correlation_id.clone()),
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
        .map_err(|e| {
            AgentRunnerError::Internal(format!("Failed to encode tool processing result: {e}"))
        })
    }

    fn sweep_sessions(&mut self, now_ms: Option<i64>) -> Result<u32, AgentRunnerError> {
        let now = now_ms.unwrap_or_else(now_unix_ms);
        self.sweep_idle_sessions(now)
    }
}

fn runtime() -> &'static Mutex<AgentRunnerRuntime> {
    static RUNTIME: OnceLock<Mutex<AgentRunnerRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(AgentRunnerRuntime::default()))
}

fn with_runtime<T>(
    f: impl FnOnce(&mut AgentRunnerRuntime) -> Result<T, AgentRunnerError>,
) -> Result<T, AgentRunnerError> {
    let mut guard = runtime().lock().map_err(|_| {
        wasm_log("runtime", LogLevel::Error, "Runtime lock poisoned");
        AgentRunnerError::Internal("AgentRunner runtime lock poisoned".to_string())
    })?;
    f(&mut guard)
}

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_session_id() -> String {
    let ts_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1_000);
    let seq = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session-{ts_ns}-{seq}")
}

pub(super) fn now_unix_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn init(config_json: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.configure(config_json))
}

pub fn set_context_policy(policy_json: &str) -> Result<bool, AgentRunnerError> {
    with_runtime(|rt| rt.set_context_policy(policy_json))
}

pub fn prepare_user_turn(request_json: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.prepare_user_turn(request_json))
}

pub fn append_llm_chunk(
    session_id: &str,
    chunk: &str,
    correlation_id: Option<&str>,
) -> Result<bool, AgentRunnerError> {
    with_runtime(|rt| rt.append_llm_chunk(session_id, chunk, correlation_id.map(|v| v.to_string())))
}

pub fn commit_llm_response(
    prepared_turn_json: &str,
    llm_response_json: &str,
) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.commit_llm_response(prepared_turn_json, llm_response_json))
}

pub fn commit_llm_stream(prepared_turn_json: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.commit_llm_stream(prepared_turn_json))
}

pub fn process_llm_response_for_session(
    session_id: &str,
    llm_response_json: &str,
) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.process_llm_response(session_id, llm_response_json))
}

pub fn process_tool_result_for_session(
    session_id: &str,
    tool_result_json: &str,
) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.process_tool_result(session_id, tool_result_json))
}

pub fn drain_events(session_id: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.drain_events(session_id))
}

pub fn get_telemetry_snapshot(session_id: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.telemetry_snapshot(session_id))
}

pub fn get_slo_snapshot(session_id: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.slo_snapshot(session_id))
}

pub fn get_state(session_id: &str) -> Result<String, AgentRunnerError> {
    with_runtime(|rt| {
        let Some(state) = rt.sessions.get(session_id) else {
            if rt.archived_sessions.contains_key(session_id) {
                wasm_log(
                    session_id,
                    LogLevel::Warn,
                    "get_state called on archived session",
                );
                return Err(AgentRunnerError::SessionArchived(format!(
                    "Session '{session_id}' is archived in host storage"
                )));
            }
            wasm_log(session_id, LogLevel::Error, "get_state: session not found");
            return Err(AgentRunnerError::SessionNotFound(session_id.to_string()));
        };
        state.state.to_json().map_err(AgentRunnerError::from)
    })
}

pub fn reset_session(session_id: &str) -> Result<bool, AgentRunnerError> {
    with_runtime(|rt| {
        let removed = rt.sessions.remove(session_id).is_some();
        rt.archived_sessions.remove(session_id);
        rt.pending_events.remove(session_id);
        rt.pending_event_seq.remove(session_id);
        Ok(removed)
    })
}

/// Register MCP tool definitions so WASM can validate LLM-driven tool calls.
///
/// `tools_json` must be a JSON array of `ToolDefinition` objects.  The call
/// replaces the entire registry; pass an empty array to clear it.  Returns the
/// number of tools registered.
pub fn register_tools(tools_json: &str) -> Result<u32, AgentRunnerError> {
    with_runtime(|rt| rt.register_tools(tools_json))
}

/// Returns a formatted tool-list block suitable for injection into a system
/// prompt, or an empty string when no tools have been registered.
pub fn get_tools_prompt() -> Result<String, AgentRunnerError> {
    with_runtime(|rt| rt.get_tools_prompt())
}

/// Trigger idle-timeout sweep manually (useful for deterministic tests and host schedulers).
pub fn sweep_idle_sessions(now_unix_ms: Option<i64>) -> Result<u32, AgentRunnerError> {
    with_runtime(|rt| rt.sweep_sessions(now_unix_ms))
}
