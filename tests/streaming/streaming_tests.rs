use antikythera_core::cli::Cli;
use antikythera_core::{
    AgentEvent, BufferPolicy, ClientInputStream, InMemoryStreamingResponse, StreamingBuffer,
    StreamingPhase2Options, StreamingRequest, StreamingResponse,
};
use antikythera_sdk::agents::{StreamingModeOption, StreamingOptions};
use clap::Parser;

#[test]
fn cli_stream_flag_defaults_to_false() {
    let cli = Cli::try_parse_from(["mcp"]).expect("cli parse should succeed");
    assert!(!cli.stream);
}

#[test]
fn cli_stream_flag_can_be_enabled() {
    let cli = Cli::try_parse_from(["mcp", "--stream"]).expect("cli parse should succeed");
    assert!(cli.stream);
}

#[test]
fn sdk_streaming_options_convert_to_core_request() {
    let options = StreamingOptions {
        mode: StreamingModeOption::Event,
        include_final_response: false,
        max_buffered_events: Some(32),
    };

    let request = options.to_streaming_request();
    assert_eq!(request.mode, antikythera_core::StreamingMode::Event);
    assert!(!request.include_final_response);
    assert_eq!(request.max_buffered_events, Some(32));
}

#[test]
fn sdk_streaming_options_validate_positive_buffer() {
    let invalid = StreamingOptions {
        mode: StreamingModeOption::Mixed,
        include_final_response: true,
        max_buffered_events: Some(0),
    };

    let error = invalid
        .validate()
        .expect_err("max_buffered_events=0 should be rejected");
    assert!(error.contains("max_buffered_events"));
}

// Phase 2 integration tests

#[test]
fn phase2_buffered_policy_batches_tool_result_events() {
    let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 3 });

    let ready1 = buf.push(AgentEvent::ToolResult {
        tool_name: "grep".to_string(),
        chunk: "line 1".to_string(),
        is_final: false,
    });
    let ready2 = buf.push(AgentEvent::ToolResult {
        tool_name: "grep".to_string(),
        chunk: "line 2".to_string(),
        is_final: false,
    });
    let ready3 = buf.push(AgentEvent::ToolResult {
        tool_name: "grep".to_string(),
        chunk: "line 3".to_string(),
        is_final: true,
    });

    assert!(!ready1);
    assert!(!ready2);
    assert!(ready3, "must be ready at threshold");

    let batch = buf.flush();
    assert_eq!(batch.len(), 3);
    assert_eq!(buf.flushed_total(), 3);
    assert_eq!(buf.pending_count(), 0);
}

#[test]
fn phase2_client_input_stream_pipeline_produces_complete_payload() {
    let sentences = ["The ", "quick ", "brown ", "fox"];
    let mut stream = ClientInputStream::new();
    for s in sentences {
        stream.push_chunk(s);
    }
    stream.complete();

    assert!(stream.is_complete());
    assert_eq!(stream.pending_count(), 4);
    assert_eq!(stream.collect_all(), "The quick brown fox");
}

#[test]
fn phase2_streaming_request_with_options_filters_summaries() {
    let request = StreamingRequest {
        phase2: Some(StreamingPhase2Options {
            include_summaries: false,
            ..StreamingPhase2Options::default()
        }),
        ..StreamingRequest::default()
    };
    let mut resp = InMemoryStreamingResponse::new(request);

    resp.push_event(AgentEvent::Summary {
        chunk: "compressed context".to_string(),
        is_final: true,
        original_message_count: 20,
    });
    resp.push_event(AgentEvent::ToolResult {
        tool_name: "search".to_string(),
        chunk: "result".to_string(),
        is_final: true,
    });
    resp.push_event(AgentEvent::Completed);

    let snapshot = resp.snapshot();
    assert_eq!(snapshot.events.len(), 2);
    assert!(matches!(&snapshot.events[0], AgentEvent::ToolResult { .. }));
    assert_eq!(snapshot.events[1], AgentEvent::Completed);
}

#[test]
fn phase2_streaming_request_phase2_absent_passes_all_events() {
    let request = StreamingRequest::default();
    let mut resp = InMemoryStreamingResponse::new(request);

    resp.push_event(AgentEvent::Summary {
        chunk: "summary".to_string(),
        is_final: true,
        original_message_count: 4,
    });
    resp.push_event(AgentEvent::ToolResult {
        tool_name: "calc".to_string(),
        chunk: "42".to_string(),
        is_final: true,
    });

    let snapshot = resp.snapshot();
    assert_eq!(snapshot.events.len(), 2);
}
