//! Streaming primitives for token, agent-event, tool-result, and summary output.

pub mod buffer;
pub mod input;
pub mod request;
pub mod response;
pub mod types;

pub use buffer::*;
pub use input::*;
pub use request::*;
pub use response::*;
pub use types::*;

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
