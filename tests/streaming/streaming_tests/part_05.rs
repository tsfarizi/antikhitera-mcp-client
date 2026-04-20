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
