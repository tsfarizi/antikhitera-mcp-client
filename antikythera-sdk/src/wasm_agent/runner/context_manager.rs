use antikythera_log::LogLevel;

use super::AgentRunnerRuntime;
use crate::wasm_agent::types::{AgentState, ContextPolicy, ContextSummary, TruncationStrategy};

impl AgentRunnerRuntime {
    pub(super) fn maybe_update_summary(
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
                let keep_head = (retain / 3).max(1);
                let keep_tail = retain.saturating_sub(keep_head).max(1);
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
        super::wasm_log(
            &state.session_id,
            LogLevel::Debug,
            &format!(
                "Context summary: {} source messages summarized, {} retained",
                summary.source_messages, retain
            ),
        );
        Some(summary)
    }
}
