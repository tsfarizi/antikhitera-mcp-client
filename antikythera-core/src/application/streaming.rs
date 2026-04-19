//! Streaming primitives for token, agent-event, tool-result, and summary output.
//!
//! ## Phase 1 — Token / Event streaming
//!
//! Basic [`StreamingRequest`], [`AgentEventStream`], and [`InMemoryStreamingResponse`]
//! for surfacing incremental tokens and structured agent lifecycle events.
//!
//! ## Phase 2 — Advanced Streaming
//!
//! Phase 2 extends the surface with:
//! - [`AgentEvent::ToolResult`] — streaming chunks of tool output
//! - [`AgentEvent::Summary`] — streaming context-management summary output
//! - [`BufferPolicy`] / [`StreamingBuffer`] — explicit buffered vs. unbuffered flush control
//! - [`ClientInputStream`] — client-side input chunking for large payloads
//! - [`StreamingPhase2Options`] — opt-in Phase 2 configuration embedded in [`StreamingRequest`]
//!
//! All Phase 1 APIs remain fully backward compatible; Phase 2 features are opt-in
//! via the `phase2` field on [`StreamingRequest`].
//!
//! # Quick start (Phase 2)
//!
//! ```
//! use antikythera_core::streaming::{
//!     AgentEvent, BufferPolicy, ClientInputStream, StreamingBuffer,
//!     StreamingPhase2Options, StreamingRequest,
//! };
//!
//! // Client-side input stream
//! let mut input = ClientInputStream::new();
//! input.push_chunk("Hello, ");
//! input.push_chunk("world!");
//! input.complete();
//! assert_eq!(input.collect_all(), "Hello, world!");
//!
//! // Buffered event buffer — flushes every 2 events
//! let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 2 });
//! buf.push(AgentEvent::Completed);
//! assert_eq!(buf.pending_count(), 1);
//! buf.push(AgentEvent::Completed);
//! let batch = buf.flush();
//! assert_eq!(batch.len(), 2);
//! ```

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
    /// Optional Phase 2 options.  When `None` the stream behaves exactly as Phase 1.
    #[serde(default)]
    pub phase2: Option<StreamingPhase2Options>,
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
            phase2: None,
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

    /// Returns a reference to the Phase 2 options if present.
    pub fn phase2_opts(&self) -> Option<&StreamingPhase2Options> {
        self.phase2.as_ref()
    }

    /// Returns `true` when Phase 2 features are active.
    pub fn is_phase2(&self) -> bool {
        self.phase2.is_some()
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
    // ── Phase 2 variants ─────────────────────────────────────────────────────
    /// A streaming chunk of tool-execution output.
    ///
    /// Multiple `ToolResult` events share the same `tool_name`; `is_final`
    /// signals the last chunk for that invocation.
    ToolResult {
        tool_name: String,
        chunk: String,
        is_final: bool,
    },
    /// A streaming chunk of context-management summarisation output.
    ///
    /// `original_message_count` reports how many messages were condensed.
    Summary {
        chunk: String,
        is_final: bool,
        original_message_count: usize,
    },
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

// ==================== Phase 2: Advanced Streaming Types ====================

/// Buffer flush policy controlling when accumulated events are yielded.
///
/// # Examples
/// ```
/// use antikythera_core::streaming::{AgentEvent, BufferPolicy, StreamingBuffer};
///
/// let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 3 });
/// buf.push(AgentEvent::Completed);
/// buf.push(AgentEvent::Completed);
/// assert_eq!(buf.pending_count(), 2);
/// buf.push(AgentEvent::Completed);
/// let batch = buf.flush();
/// assert_eq!(batch.len(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferPolicy {
    /// Every event is yielded immediately — lowest latency, highest per-event overhead.
    Unbuffered,
    /// Events accumulate until `flush_threshold` is reached, then the whole batch is
    /// flushed at once. A `flush_threshold` of `0` is treated as `1`.
    Buffered { flush_threshold: usize },
}

impl Default for BufferPolicy {
    fn default() -> Self {
        Self::Unbuffered
    }
}

/// Accumulates [`AgentEvent`]s and flushes them according to a [`BufferPolicy`].
///
/// Use [`StreamingBuffer::push`] to enqueue events. The method returns `true`
/// when the buffer is ready to be flushed. Call [`StreamingBuffer::flush`] to
/// drain the batch.
///
/// # Examples
/// ```
/// use antikythera_core::streaming::{AgentEvent, BufferPolicy, StreamingBuffer};
///
/// let mut buf = StreamingBuffer::new(BufferPolicy::Unbuffered);
/// let ready = buf.push(AgentEvent::Completed);
/// assert!(ready); // unbuffered is always ready
/// let batch = buf.flush();
/// assert_eq!(batch.len(), 1);
/// ```
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

/// Phase 2 streaming options. Embed in [`StreamingRequest::phase2`] to activate
/// advanced features without breaking Phase 1 consumers.
///
/// All fields default to permissive values (include everything, unbuffered).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamingPhase2Options {
    /// Flush policy for buffering events before delivery.
    #[serde(default)]
    pub buffer_policy: BufferPolicy,
    /// Forward [`AgentEvent::ToolResult`] chunks to the host.
    #[serde(default = "default_true")]
    pub include_tool_results: bool,
    /// Forward [`AgentEvent::Summary`] chunks from context management to the host.
    #[serde(default = "default_true")]
    pub include_summaries: bool,
}

impl Default for StreamingPhase2Options {
    fn default() -> Self {
        Self {
            buffer_policy: BufferPolicy::Unbuffered,
            include_tool_results: true,
            include_summaries: true,
        }
    }
}

/// Client-side input stream for large request payloads.
///
/// Rather than providing a single monolithic input string, hosts push incremental
/// chunks. This enables pipelining of large contexts, file uploads, or piped
/// stdin without blocking until the entire payload is ready.
///
/// # Examples
/// ```
/// use antikythera_core::streaming::ClientInputStream;
///
/// let mut stream = ClientInputStream::new();
/// stream.push_chunk("The quick ");
/// stream.push_chunk("brown fox");
/// stream.complete();
/// assert_eq!(stream.total_chars_pushed(), 19);
/// assert_eq!(stream.collect_all(), "The quick brown fox");
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClientInputStream {
    chunks: VecDeque<String>,
    is_complete: bool,
    total_chars_pushed: usize,
}

impl ClientInputStream {
    /// Create an empty, open input stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push the next input chunk.
    ///
    /// # Panics
    /// Panics if the stream has already been completed via [`complete`](Self::complete).
    pub fn push_chunk(&mut self, chunk: impl Into<String>) {
        assert!(
            !self.is_complete,
            "cannot push to a completed ClientInputStream"
        );
        let s = chunk.into();
        self.total_chars_pushed += s.chars().count();
        self.chunks.push_back(s);
    }

    /// Signal end-of-stream. No further chunks may be pushed after this call.
    pub fn complete(&mut self) {
        self.is_complete = true;
    }

    /// Whether the producer has signalled end-of-stream.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Pop the next available chunk in FIFO order, or `None` if the buffer is empty.
    pub fn next_chunk(&mut self) -> Option<String> {
        self.chunks.pop_front()
    }

    /// Number of chunks currently waiting to be consumed.
    pub fn pending_count(&self) -> usize {
        self.chunks.len()
    }

    /// Total unicode codepoints that have been pushed into this stream.
    pub fn total_chars_pushed(&self) -> usize {
        self.total_chars_pushed
    }

    /// Consume the stream and concatenate all remaining chunks into a single `String`.
    pub fn collect_all(mut self) -> String {
        let mut result = String::with_capacity(self.total_chars_pushed);
        while let Some(chunk) = self.chunks.pop_front() {
            result.push_str(&chunk);
        }
        result
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

    // ── Phase 2 unit tests ────────────────────────────────────────────────────

    #[test]
    fn buffer_policy_unbuffered_flushes_immediately() {
        let mut buf = StreamingBuffer::new(BufferPolicy::Unbuffered);
        let ready = buf.push(AgentEvent::Completed);
        assert!(ready, "unbuffered must signal ready after every push");
    }

    #[test]
    fn buffer_policy_buffered_not_ready_until_threshold() {
        let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 3 });
        assert!(!buf.push(AgentEvent::Completed));
        assert!(!buf.push(AgentEvent::Completed));
        let ready = buf.push(AgentEvent::Completed);
        assert!(ready, "should be ready exactly at threshold");
    }

    #[test]
    fn buffer_policy_flush_clears_pending_and_returns_all_events() {
        let mut buf = StreamingBuffer::new(BufferPolicy::Unbuffered);
        buf.push(AgentEvent::State {
            state: "a".into(),
            detail: None,
        });
        buf.push(AgentEvent::Completed);
        let batch = buf.flush();
        assert_eq!(batch.len(), 2);
        assert_eq!(buf.pending_count(), 0);
    }

    #[test]
    fn buffer_policy_flushed_total_accumulates_across_flushes() {
        let mut buf = StreamingBuffer::new(BufferPolicy::Unbuffered);
        buf.push(AgentEvent::Completed);
        buf.flush();
        buf.push(AgentEvent::Completed);
        buf.push(AgentEvent::Completed);
        buf.flush();
        assert_eq!(buf.flushed_total(), 3);
    }

    #[test]
    fn buffer_policy_zero_threshold_treated_as_one() {
        let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 0 });
        let ready = buf.push(AgentEvent::Completed);
        assert!(
            ready,
            "flush_threshold=0 should be clamped to 1 → ready after first push"
        );
    }

    #[test]
    fn client_input_stream_collect_all_concatenates_in_order() {
        let mut stream = ClientInputStream::new();
        stream.push_chunk("Hello, ");
        stream.push_chunk("world!");
        stream.complete();
        assert_eq!(stream.collect_all(), "Hello, world!");
    }

    #[test]
    fn client_input_stream_pending_count_decrements_on_next_chunk() {
        let mut stream = ClientInputStream::new();
        stream.push_chunk("a");
        stream.push_chunk("b");
        assert_eq!(stream.pending_count(), 2);
        let _ = stream.next_chunk();
        assert_eq!(stream.pending_count(), 1);
    }

    #[test]
    fn client_input_stream_total_chars_counts_unicode_codepoints() {
        let mut stream = ClientInputStream::new();
        stream.push_chunk("café"); // 4 codepoints
        stream.push_chunk("🦀"); // 1 codepoint
        assert_eq!(stream.total_chars_pushed(), 5);
    }

    #[test]
    #[should_panic(expected = "cannot push to a completed ClientInputStream")]
    fn client_input_stream_push_after_complete_panics() {
        let mut stream = ClientInputStream::new();
        stream.complete();
        stream.push_chunk("oops");
    }

    #[test]
    fn agent_event_tool_result_serialises_with_kind_tag() {
        let event = AgentEvent::ToolResult {
            tool_name: "search".to_string(),
            chunk: "result chunk".to_string(),
            is_final: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"tool_result\""));
        assert!(json.contains("\"is_final\":true"));
    }

    #[test]
    fn agent_event_summary_serialises_with_kind_tag() {
        let event = AgentEvent::Summary {
            chunk: "summary text".to_string(),
            is_final: false,
            original_message_count: 12,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"summary\""));
        assert!(json.contains("\"original_message_count\":12"));
    }

    #[test]
    fn in_memory_response_filters_tool_results_when_disabled() {
        let request = StreamingRequest {
            phase2: Some(StreamingPhase2Options {
                include_tool_results: false,
                ..StreamingPhase2Options::default()
            }),
            ..StreamingRequest::default()
        };
        let mut response = InMemoryStreamingResponse::new(request);
        response.push_event(AgentEvent::ToolResult {
            tool_name: "grep".to_string(),
            chunk: "match".to_string(),
            is_final: true,
        });
        response.push_event(AgentEvent::Completed);
        let snapshot = response.snapshot();
        // ToolResult filtered out; only Completed present
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0], AgentEvent::Completed);
    }

    #[test]
    fn in_memory_response_filters_summaries_when_disabled() {
        let request = StreamingRequest {
            phase2: Some(StreamingPhase2Options {
                include_summaries: false,
                ..StreamingPhase2Options::default()
            }),
            ..StreamingRequest::default()
        };
        let mut response = InMemoryStreamingResponse::new(request);
        response.push_event(AgentEvent::Summary {
            chunk: "condensed".to_string(),
            is_final: true,
            original_message_count: 5,
        });
        response.push_event(AgentEvent::Completed);
        let snapshot = response.snapshot();
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0], AgentEvent::Completed);
    }

    #[test]
    fn streaming_request_phase2_opts_returns_none_by_default() {
        let request = StreamingRequest::default();
        assert!(!request.is_phase2());
        assert!(request.phase2_opts().is_none());
    }

    #[test]
    fn streaming_phase2_options_default_includes_all() {
        let opts = StreamingPhase2Options::default();
        assert!(opts.include_tool_results);
        assert!(opts.include_summaries);
        assert_eq!(opts.buffer_policy, BufferPolicy::Unbuffered);
    }

    #[test]
    fn event_stream_push_tool_result_and_summary_helpers() {
        let mut stream = AgentEventStream::new();
        stream.push_tool_result("search", "chunk 1", false);
        stream.push_tool_result("search", "chunk 2", true);
        stream.push_summary("condensed text", true, 8);

        let events = stream.drain();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[0],
            AgentEvent::ToolResult {
                is_final: false,
                ..
            }
        ));
        assert!(matches!(
            &events[1],
            AgentEvent::ToolResult { is_final: true, .. }
        ));
        assert!(matches!(
            &events[2],
            AgentEvent::Summary {
                original_message_count: 8,
                ..
            }
        ));
    }
}
