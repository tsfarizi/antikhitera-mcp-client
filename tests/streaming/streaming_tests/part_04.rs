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

