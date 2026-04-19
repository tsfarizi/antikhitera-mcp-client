//! Streaming primitives for token and agent-event output.
//!
//! This module defines a lightweight phase-1 streaming surface that hosts can
//! use without changing existing non-streaming APIs.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Streaming mode requested by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamingMode {
    Token,
    Event,
    #[default]
    Mixed,
}

/// Streaming request describing what kind of incremental output is wanted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamingRequest {
    #[serde(default)]
    pub mode: StreamingMode,
    #[serde(default = "default_true")]
    pub include_final_response: bool,
    #[serde(default)]
    pub max_buffered_events: Option<usize>,
}

const fn default_true() -> bool {
    true
}

impl Default for StreamingRequest {
    fn default() -> Self {
        Self {
            mode: StreamingMode::Mixed,
            include_final_response: true,
            max_buffered_events: None,
        }
    }
}

impl StreamingRequest {
    /// Returns true when token chunks should be surfaced.
    pub fn wants_tokens(&self) -> bool {
        matches!(self.mode, StreamingMode::Token | StreamingMode::Mixed)
    }

    /// Returns true when structured agent events should be surfaced.
    pub fn wants_events(&self) -> bool {
        matches!(self.mode, StreamingMode::Event | StreamingMode::Mixed)
    }
}

/// Intermediate event emitted by the agent pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    Token {
        content: String,
    },
    Tool {
        tool_name: String,
        phase: ToolEventPhase,
    },
    State {
        state: String,
        detail: Option<String>,
    },
    Completed,
}

/// Tool event phase for structured tool events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEventPhase {
    Started,
    Finished,
}

/// Bounded in-memory stream of agent events.
#[derive(Debug, Clone, Default)]
pub struct AgentEventStream {
    max_buffered_events: Option<usize>,
    events: VecDeque<AgentEvent>,
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

/// Snapshot returned by in-memory streaming responders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StreamingSnapshot {
    pub mode: StreamingMode,
    pub tokens: Vec<String>,
    pub events: Vec<AgentEvent>,
    pub final_response: Option<String>,
}

/// Provider-facing abstraction for streaming responses.
pub trait StreamingResponse: Send {
    fn request(&self) -> &StreamingRequest;
    fn push_token(&mut self, token: String);
    fn push_event(&mut self, event: AgentEvent);
    fn set_final_response(&mut self, response: String);
    fn snapshot(&self) -> StreamingSnapshot;
}

/// In-memory streaming collector used by tests and host adapters.
#[derive(Debug, Clone)]
pub struct InMemoryStreamingResponse {
    request: StreamingRequest,
    tokens: Vec<String>,
    events: AgentEventStream,
    final_response: Option<String>,
}

impl InMemoryStreamingResponse {
    /// Build an in-memory stream collector from a request.
    pub fn new(request: StreamingRequest) -> Self {
        Self {
            events: AgentEventStream::with_max_buffered_events(request.max_buffered_events),
            request,
            tokens: Vec::new(),
            final_response: None,
        }
    }
}

impl StreamingResponse for InMemoryStreamingResponse {
    fn request(&self) -> &StreamingRequest {
        &self.request
    }

    fn push_token(&mut self, token: String) {
        if self.request.wants_tokens() {
            self.tokens.push(token.clone());
        }
        if self.request.wants_events() {
            self.events.push(AgentEvent::Token { content: token });
        }
    }

    fn push_event(&mut self, event: AgentEvent) {
        if self.request.wants_events() {
            self.events.push(event);
        }
    }

    fn set_final_response(&mut self, response: String) {
        if self.request.include_final_response {
            self.final_response = Some(response);
        }
    }

    fn snapshot(&self) -> StreamingSnapshot {
        StreamingSnapshot {
            mode: self.request.mode,
            tokens: self.tokens.clone(),
            events: self.events.events.iter().cloned().collect(),
            final_response: self.final_response.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_request_is_mixed_with_final_response() {
        let request = StreamingRequest::default();
        assert_eq!(request.mode, StreamingMode::Mixed);
        assert!(request.include_final_response);
        assert!(request.wants_tokens());
        assert!(request.wants_events());
    }

    #[test]
    fn token_mode_requests_only_tokens() {
        let request = StreamingRequest {
            mode: StreamingMode::Token,
            ..StreamingRequest::default()
        };
        assert!(request.wants_tokens());
        assert!(!request.wants_events());
    }

    #[test]
    fn event_mode_requests_only_events() {
        let request = StreamingRequest {
            mode: StreamingMode::Event,
            ..StreamingRequest::default()
        };
        assert!(!request.wants_tokens());
        assert!(request.wants_events());
    }

    #[test]
    fn event_stream_push_and_pop_preserve_fifo_order() {
        let mut stream = AgentEventStream::new();
        stream.push_state("routing", None);
        stream.push_tool("search", ToolEventPhase::Started);

        assert_eq!(
            stream.pop_next(),
            Some(AgentEvent::State {
                state: "routing".to_string(),
                detail: None,
            })
        );
        assert_eq!(
            stream.pop_next(),
            Some(AgentEvent::Tool {
                tool_name: "search".to_string(),
                phase: ToolEventPhase::Started,
            })
        );
        assert_eq!(stream.pop_next(), None);
    }

    #[test]
    fn bounded_event_stream_drops_oldest_events() {
        let mut stream = AgentEventStream::with_max_buffered_events(Some(2));
        stream.push_state("s1", None);
        stream.push_state("s2", None);
        stream.push_state("s3", None);

        let events = stream.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            AgentEvent::State {
                state: "s2".to_string(),
                detail: None,
            }
        );
    }

    #[test]
    fn zero_buffer_keeps_no_events() {
        let mut stream = AgentEventStream::with_max_buffered_events(Some(0));
        stream.push_state("ignored", None);
        assert!(stream.is_empty());
    }

    #[test]
    fn token_mode_response_collects_tokens_only() {
        let request = StreamingRequest {
            mode: StreamingMode::Token,
            ..StreamingRequest::default()
        };
        let mut response = InMemoryStreamingResponse::new(request);
        response.push_token("hel".to_string());
        response.push_token("lo".to_string());
        response.push_event(AgentEvent::Completed);

        let snapshot = response.snapshot();
        assert_eq!(snapshot.tokens, vec!["hel", "lo"]);
        assert!(snapshot.events.is_empty());
    }

    #[test]
    fn event_mode_response_collects_events_only() {
        let request = StreamingRequest {
            mode: StreamingMode::Event,
            ..StreamingRequest::default()
        };
        let mut response = InMemoryStreamingResponse::new(request);
        response.push_token("chunk".to_string());
        response.push_event(AgentEvent::Completed);

        let snapshot = response.snapshot();
        assert!(snapshot.tokens.is_empty());
        assert_eq!(snapshot.events.len(), 2);
    }

    #[test]
    fn mixed_mode_collects_tokens_and_events() {
        let mut response = InMemoryStreamingResponse::new(StreamingRequest::default());
        response.push_token("chunk".to_string());

        let snapshot = response.snapshot();
        assert_eq!(snapshot.tokens, vec!["chunk"]);
        assert_eq!(snapshot.events.len(), 1);
    }

    #[test]
    fn final_response_respects_include_flag() {
        let mut response = InMemoryStreamingResponse::new(StreamingRequest {
            include_final_response: false,
            ..StreamingRequest::default()
        });
        response.set_final_response("done".to_string());
        assert_eq!(response.snapshot().final_response, None);
    }

    #[test]
    fn event_stream_complete_adds_completed_event() {
        let mut stream = AgentEventStream::new();
        stream.complete();
        assert_eq!(stream.pop_next(), Some(AgentEvent::Completed));
    }
}
