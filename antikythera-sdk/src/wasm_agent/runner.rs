use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::processor::{build_llm_messages, process_llm_response, process_tool_result, validate_tool_call};
use super::types::{
    AgentAction, AgentConfig, AgentMessage, AgentState, ContextPolicy, ContextSummary, SloSnapshot,
    StreamEvent, StreamEventKind, TelemetryCounters, TelemetrySnapshot, ToolCall, ToolRegistry,
    ToolResult, TruncationStrategy,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunnerConfigInput {
    pub max_steps: Option<u32>,
    pub verbose: Option<bool>,
    pub auto_execute_tools: Option<bool>,
    pub session_timeout_secs: Option<u32>,
    pub max_in_memory_sessions: Option<usize>,
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
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContextPolicyUpdateInput {
    pub policy: ContextPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchivedSessionRecord {
    pub archived_at_ms: i64,
    pub reason: String,
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

struct AgentRunnerRuntime {
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
    fn p95(values: &[u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let idx = ((sorted.len() - 1) * 95) / 100;
        sorted[idx]
    }

    fn emit_pending_event(
        &mut self,
        session_id: &str,
        kind: StreamEventKind,
        correlation_id: Option<String>,
        payload: serde_json::Value,
    ) {
        let seq_entry = self
            .pending_event_seq
            .entry(session_id.to_string())
            .or_insert(0);
        *seq_entry += 1;
        self.pending_events
            .entry(session_id.to_string())
            .or_default()
            .push(StreamEvent {
                seq: *seq_entry,
                session_id: session_id.to_string(),
                step: 0,
                correlation_id,
                kind,
                payload,
            });
    }

    fn archive_session(
        &mut self,
        session_id: &str,
        reason: &str,
        correlation_id: Option<String>,
    ) -> Result<bool, String> {
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

    fn sweep_idle_sessions(&mut self, now_ms: i64) -> Result<u32, String> {
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
        Ok(archived)
    }

    fn enforce_capacity(
        &mut self,
        protected_session_id: Option<&str>,
        correlation_id: Option<String>,
    ) -> Result<u32, String> {
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

    fn register_tools(&mut self, tools_json: &str) -> Result<u32, String> {
        self.known_tools = ToolRegistry::from_json(tools_json)?;
        Ok(self.known_tools.len() as u32)
    }

    fn get_tools_prompt(&self) -> Result<String, String> {
        let block = self.known_tools.to_prompt_block().unwrap_or_default();
        Ok(block)
    }

    fn configure(&mut self, config_json: &str) -> Result<String, String> {
        let input: RunnerConfigInput =
            serde_json::from_str(config_json).map_err(|e| format!("Invalid config-json: {e}"))?;

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
        self.sessions
            .entry(session_id.clone())
            .or_insert_with(|| SessionRuntime::new(config));

        let _ = self.enforce_capacity(Some(&session_id), None)?;

        Ok(session_id)
    }

    fn set_context_policy(&mut self, policy_json: &str) -> Result<bool, String> {
        let input: ContextPolicyUpdateInput = serde_json::from_str(policy_json)
            .map_err(|e| format!("Invalid context-policy-json: {e}"))?;
        self.default_config.context_policy = input.policy;
        Ok(true)
    }

    fn maybe_update_summary(
        state: &mut AgentState,
        policy: &ContextPolicy,
    ) -> Option<ContextSummary> {
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
        let input: PrepareUserTurnInput =
            serde_json::from_str(request_json).map_err(|e| format!("Invalid request-json: {e}"))?;

        let now_ms = now_unix_ms();
        let _ = self.sweep_idle_sessions(now_ms)?;

        // Snapshot the tool block before the mutable session borrow to avoid borrow conflict.
        let tool_block_snapshot = self.known_tools.to_prompt_block();

        let session_id = input.session_id.clone().unwrap_or_else(new_session_id);

        if !self.sessions.contains_key(&session_id)
            && self.archived_sessions.contains_key(&session_id)
        {
            let archived = self.archived_sessions.get(&session_id).cloned().unwrap_or(
                ArchivedSessionRecord {
                    archived_at_ms: now_ms,
                    reason: "unknown".to_string(),
                },
            );
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
            return Err(format!(
                "Session '{session_id}' archived and not in RAM. Host must load persisted state then call hydrate_session"
            ));
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
            messages_json: serde_json::to_string(&messages)
                .map_err(|e| format!("Failed to encode messages_json: {e}"))?,
        };

        let encoded =
            serde_json::to_string(&prepared).map_err(|e| format!("Failed to encode prepared turn: {e}"))?;

        let _ = self.enforce_capacity(
            Some(&prepared.session_id),
            prepared.correlation_id.clone(),
        )?;

        Ok(encoded)
    }

    fn append_llm_chunk(
        &mut self,
        session_id: &str,
        chunk: &str,
        correlation_id: Option<String>,
    ) -> Result<bool, String> {
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
    ) -> Result<String, String> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let started = Instant::now();
        let prepared: PreparedTurn = serde_json::from_str(prepared_turn_json)
            .map_err(|e| format!("Invalid prepared-turn-json: {e}"))?;

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
                    return Err(validation_err.to_string());
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

    fn process_llm_response(
        &mut self,
        session_id: &str,
        llm_response_json: &str,
    ) -> Result<String, String> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
        let action = process_llm_response(&mut runtime.state, llm_response_json)?;
        serde_json::to_string(&action).map_err(|e| format!("Failed to encode action: {e}"))
    }

    fn process_tool_result(
        &mut self,
        session_id: &str,
        tool_result_json: &str,
    ) -> Result<String, String> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let input: ToolResultInput = serde_json::from_str(tool_result_json)
            .map_err(|e| format!("Invalid tool-result-json: {e}"))?;

        let output: serde_json::Value = serde_json::from_str(&input.output_json)
            .map_err(|e| format!("Invalid tool output_json: {e}"))?;

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
        .map_err(|e| format!("Failed to encode tool processing result: {e}"))
    }

    fn drain_events(&mut self, session_id: &str) -> Result<String, String> {
        let mut events = Vec::new();
        if let Some(runtime) = self.sessions.get_mut(session_id) {
            events.extend(std::mem::take(&mut runtime.events));
        }
        if let Some(pending) = self.pending_events.remove(session_id) {
            events.extend(pending);
        }

        if events.is_empty()
            && !self.sessions.contains_key(session_id)
            && !self.archived_sessions.contains_key(session_id)
        {
            return Err(format!("Session not found: {session_id}"));
        }

        serde_json::to_string(&events).map_err(|e| format!("Failed to encode events: {e}"))
    }

    fn telemetry_snapshot(&mut self, session_id: &str) -> Result<String, String> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
        runtime.emit_event(
            StreamEventKind::Telemetry,
            runtime.telemetry.correlation_id.clone(),
            serde_json::json!({"snapshot": true}),
        );
        serde_json::to_string(&runtime.telemetry)
            .map_err(|e| format!("Failed to encode telemetry snapshot: {e}"))
    }

    fn slo_snapshot(&mut self, session_id: &str) -> Result<String, String> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;

        let commits = runtime.telemetry.counters.llm_commits as f64;
        let tool_results = runtime.telemetry.counters.tool_results as f64;
        let retries = runtime.telemetry.counters.llm_retries as f64;

        let success_rate = if commits > 0.0 {
            runtime.telemetry.counters.final_responses as f64 / commits
        } else {
            0.0
        };
        let tool_error_rate = if tool_results > 0.0 {
            runtime.telemetry.counters.tool_errors as f64 / tool_results
        } else {
            0.0
        };
        let retry_ratio = if commits > 0.0 { retries / commits } else { 0.0 };

        let snapshot = SloSnapshot {
            session_id: runtime.state.session_id.clone(),
            correlation_id: runtime.telemetry.correlation_id.clone(),
            success_rate,
            tool_error_rate,
            retry_ratio,
            p95_prepare_latency_ms: Self::p95(&runtime.prepare_latencies_ms),
            p95_commit_latency_ms: Self::p95(&runtime.commit_latencies_ms),
        };

        serde_json::to_string(&snapshot).map_err(|e| format!("Failed to encode SLO snapshot: {e}"))
    }

    fn hydrate_session(&mut self, session_id: &str, state_json: &str) -> Result<bool, String> {
        let mut state = AgentState::from_json(state_json)?;
        state.session_id = session_id.to_string();

        let config = state.config.clone();
        let now_ms = now_unix_ms();
        let mut runtime = SessionRuntime::new(config);
        runtime.state = state;
        runtime.touch(now_ms);

        self.sessions.insert(session_id.to_string(), runtime);
        self.archived_sessions.remove(session_id);

        self.emit_pending_event(
            session_id,
            StreamEventKind::SessionRestored,
            None,
            serde_json::json!({
                "restored_at_ms": now_ms,
                "source": "host_load_state"
            }),
        );

        Ok(true)
    }

    fn report_restore_progress(
        &mut self,
        session_id: &str,
        progress_json: &str,
    ) -> Result<bool, String> {
        let payload: serde_json::Value = serde_json::from_str(progress_json)
            .map_err(|e| format!("Invalid progress-json: {e}"))?;
        self.emit_pending_event(
            session_id,
            StreamEventKind::SessionRestoreProgress,
            None,
            payload,
        );
        Ok(true)
    }

    fn sweep_sessions(&mut self, now_ms: Option<i64>) -> Result<u32, String> {
        let now = now_ms.unwrap_or_else(now_unix_ms);
        self.sweep_idle_sessions(now)
    }
}

fn runtime() -> &'static Mutex<AgentRunnerRuntime> {
    static RUNTIME: OnceLock<Mutex<AgentRunnerRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(AgentRunnerRuntime::default()))
}

fn with_runtime<T>(
    f: impl FnOnce(&mut AgentRunnerRuntime) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = runtime()
        .lock()
        .map_err(|_| "AgentRunner runtime lock poisoned".to_string())?;
    f(&mut guard)
}

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_session_id() -> String {
    let ts_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1_000);
    let seq = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session-{ts_ns}-{seq}")
}

fn now_unix_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
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

pub fn append_llm_chunk(
    session_id: &str,
    chunk: &str,
    correlation_id: Option<&str>,
) -> Result<bool, String> {
    with_runtime(|rt| rt.append_llm_chunk(session_id, chunk, correlation_id.map(|v| v.to_string())))
}

pub fn commit_llm_response(
    prepared_turn_json: &str,
    llm_response_json: &str,
) -> Result<String, String> {
    with_runtime(|rt| rt.commit_llm_response(prepared_turn_json, llm_response_json))
}

pub fn commit_llm_stream(prepared_turn_json: &str) -> Result<String, String> {
    with_runtime(|rt| rt.commit_llm_stream(prepared_turn_json))
}

pub fn process_llm_response_for_session(
    session_id: &str,
    llm_response_json: &str,
) -> Result<String, String> {
    with_runtime(|rt| rt.process_llm_response(session_id, llm_response_json))
}

pub fn process_tool_result_for_session(
    session_id: &str,
    tool_result_json: &str,
) -> Result<String, String> {
    with_runtime(|rt| rt.process_tool_result(session_id, tool_result_json))
}

pub fn drain_events(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| rt.drain_events(session_id))
}

pub fn get_telemetry_snapshot(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| rt.telemetry_snapshot(session_id))
}

pub fn get_slo_snapshot(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| rt.slo_snapshot(session_id))
}

pub fn get_state(session_id: &str) -> Result<String, String> {
    with_runtime(|rt| {
        let Some(state) = rt.sessions.get(session_id) else {
            if rt.archived_sessions.contains_key(session_id) {
                return Err(format!(
                    "Session '{session_id}' is archived in host storage; call hydrate_session after host load"
                ));
            }
            return Err(format!("Session not found: {session_id}"));
        };
        state.state.to_json()
    })
}

pub fn reset_session(session_id: &str) -> Result<bool, String> {
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
pub fn register_tools(tools_json: &str) -> Result<u32, String> {
    with_runtime(|rt| rt.register_tools(tools_json))
}

/// Returns a formatted tool-list block suitable for injection into a system
/// prompt, or an empty string when no tools have been registered.
pub fn get_tools_prompt() -> Result<String, String> {
    with_runtime(|rt| rt.get_tools_prompt())
}

/// Restore an archived session into WASM memory after the host has loaded it.
pub fn hydrate_session(session_id: &str, state_json: &str) -> Result<bool, String> {
    with_runtime(|rt| rt.hydrate_session(session_id, state_json))
}

/// Emit stream progress updates for session restore so host/user can see load progress.
pub fn report_session_restore_progress(session_id: &str, progress_json: &str) -> Result<bool, String> {
    with_runtime(|rt| rt.report_restore_progress(session_id, progress_json))
}

/// Trigger idle-timeout sweep manually (useful for deterministic tests and host schedulers).
pub fn sweep_idle_sessions(now_unix_ms: Option<i64>) -> Result<u32, String> {
    with_runtime(|rt| rt.sweep_sessions(now_unix_ms))
}
