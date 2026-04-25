//! Streaming response collectors

use super::buffer::AgentEventStream;
use super::request::StreamingRequest;
use super::types::{AgentEvent, StreamingMode};
use serde::{Deserialize, Serialize};

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
        if !self.request.wants_events() {
            return;
        }
        // Phase 2 filtering: honour include_tool_results / include_summaries flags.
        if let Some(p2) = self.request.phase2.as_ref() {
            match &event {
                AgentEvent::ToolResult { .. } if !p2.include_tool_results => return,
                AgentEvent::Summary { .. } if !p2.include_summaries => return,
                _ => {}
            }
        }
        self.events.push(event);
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
