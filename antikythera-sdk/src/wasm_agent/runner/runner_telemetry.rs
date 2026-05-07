use antikythera_log::LogLevel;

use super::{AgentRunnerError, AgentRunnerRuntime, now_unix_ms};
use crate::wasm_agent::types::{SloSnapshot, StreamEvent, StreamEventKind};

impl AgentRunnerRuntime {
    pub fn p95(values: &[u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let idx = ((sorted.len() - 1) * 95) / 100;
        sorted[idx]
    }

    pub(super) fn emit_pending_event(
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

    pub(super) fn drain_events(&mut self, session_id: &str) -> Result<String, AgentRunnerError> {
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
            return Err(AgentRunnerError::SessionNotFound(session_id.to_string()));
        }

        serde_json::to_string(&events)
            .map_err(|e| AgentRunnerError::Internal(format!("Failed to encode events: {e}")))
    }

    pub(super) fn telemetry_snapshot(
        &mut self,
        session_id: &str,
    ) -> Result<String, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self.ensure_session(session_id);
        runtime.touch(now_unix_ms());
        super::wasm_log("tui", LogLevel::Debug, "Telemetry snapshot");
        runtime.emit_event(
            StreamEventKind::Telemetry,
            runtime.telemetry.correlation_id.clone(),
            serde_json::json!({"snapshot": true}),
        );
        serde_json::to_string(&runtime.telemetry).map_err(|e| {
            AgentRunnerError::Internal(format!("Failed to encode telemetry snapshot: {e}"))
        })
    }

    pub(super) fn slo_snapshot(&mut self, session_id: &str) -> Result<String, AgentRunnerError> {
        let _ = self.sweep_idle_sessions(now_unix_ms())?;
        let runtime = self
            .sessions
            .get(session_id)
            .ok_or_else(|| AgentRunnerError::SessionNotFound(session_id.to_string()))?;

        super::wasm_log("tui", LogLevel::Debug, "SLO snapshot");

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
        let retry_ratio = if commits > 0.0 {
            retries / commits
        } else {
            0.0
        };

        let snapshot = SloSnapshot {
            session_id: runtime.state.session_id.clone(),
            correlation_id: runtime.telemetry.correlation_id.clone(),
            success_rate,
            tool_error_rate,
            retry_ratio,
            p95_prepare_latency_ms: Self::p95(&runtime.prepare_latencies_ms),
            p95_commit_latency_ms: Self::p95(&runtime.commit_latencies_ms),
        };

        serde_json::to_string(&snapshot)
            .map_err(|e| AgentRunnerError::Internal(format!("Failed to encode SLO snapshot: {e}")))
    }
}
