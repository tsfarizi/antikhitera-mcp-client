//! Buffering and event streams

use super::types::{AgentEvent, ToolEventPhase};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Bounded in-memory stream of agent events.
#[derive(Debug, Clone, Default)]
pub struct AgentEventStream {
    pub(crate) max_buffered_events: Option<usize>,
    pub(crate) events: VecDeque<AgentEvent>,
}

impl AgentEventStream {
    /// Create an unbounded event stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a stream with a maximum number of buffered events.
    pub fn with_max_buffered_events(max_buffered_events: Option<usize>) -> Self {
        Self {
            max_buffered_events,
            events: VecDeque::new(),
        }
    }

    /// Push an event into the stream.
    pub fn push(&mut self, event: AgentEvent) {
        self.events.push_back(event);
        self.enforce_bound();
    }

    /// Push a token event.
    pub fn push_token(&mut self, content: impl Into<String>) {
        self.push(AgentEvent::Token {
            content: content.into(),
        });
    }

    /// Push a tool event.
    pub fn push_tool(&mut self, tool_name: impl Into<String>, phase: ToolEventPhase) {
        self.push(AgentEvent::Tool {
            tool_name: tool_name.into(),
            phase,
        });
    }

    /// Push a state transition event.
    pub fn push_state(&mut self, state: impl Into<String>, detail: Option<String>) {
        self.push(AgentEvent::State {
            state: state.into(),
            detail,
        });
    }

    /// Mark stream completion.
    pub fn complete(&mut self) {
        self.push(AgentEvent::Completed);
    }

    /// Push a streaming tool-result chunk.
    pub fn push_tool_result(
        &mut self,
        tool_name: impl Into<String>,
        chunk: impl Into<String>,
        is_final: bool,
    ) {
        self.push(AgentEvent::ToolResult {
            tool_name: tool_name.into(),
            chunk: chunk.into(),
            is_final,
        });
    }

    /// Push a streaming summary chunk from context management.
    pub fn push_summary(
        &mut self,
        chunk: impl Into<String>,
        is_final: bool,
        original_message_count: usize,
    ) {
        self.push(AgentEvent::Summary {
            chunk: chunk.into(),
            is_final,
            original_message_count,
        });
    }

    /// Number of currently buffered events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no events are currently buffered.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Pop the next event from the front of the stream.
    pub fn pop_next(&mut self) -> Option<AgentEvent> {
        self.events.pop_front()
    }

    /// Drain all events to a vector in FIFO order.
    pub fn drain(&mut self) -> Vec<AgentEvent> {
        self.events.drain(..).collect()
    }

    fn enforce_bound(&mut self) {
        if let Some(max) = self.max_buffered_events {
            if max == 0 {
                self.events.clear();
                return;
            }

            while self.events.len() > max {
                let _ = self.events.pop_front();
            }
        }
    }
}

/// Buffer flush policy controlling when accumulated events are yielded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BufferPolicy {
    /// Every event is yielded immediately — lowest latency, highest per-event overhead.
    #[default]
    Unbuffered,
    /// Events accumulate until `flush_threshold` is reached.
    Buffered { flush_threshold: usize },
}

/// Accumulates [`AgentEvent`]s and flushes them according to a [`BufferPolicy`].
#[derive(Debug, Clone, Default)]
pub struct StreamingBuffer {
    policy: BufferPolicy,
    pending: Vec<AgentEvent>,
    flushed_total: usize,
}

impl StreamingBuffer {
    /// Create a new buffer with the given flush policy.
    pub fn new(policy: BufferPolicy) -> Self {
        Self {
            policy,
            pending: Vec::new(),
            flushed_total: 0,
        }
    }

    /// Push an event and return `true` when the buffer should be flushed.
    pub fn push(&mut self, event: AgentEvent) -> bool {
        self.pending.push(event);
        match &self.policy {
            BufferPolicy::Unbuffered => true,
            BufferPolicy::Buffered { flush_threshold } => {
                let threshold = (*flush_threshold).max(1);
                self.pending.len() >= threshold
            }
        }
    }

    /// Drain and return all pending events. Resets the internal buffer.
    pub fn flush(&mut self) -> Vec<AgentEvent> {
        let batch = std::mem::take(&mut self.pending);
        self.flushed_total += batch.len();
        batch
    }

    /// Number of events currently waiting to be flushed.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Cumulative count of all events that have been flushed from this buffer.
    pub fn flushed_total(&self) -> usize {
        self.flushed_total
    }
}
