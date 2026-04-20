// ============================================================================
// STREAMING BUFFER TESTS
// ============================================================================

#[test]
fn test_streaming_buffer_unbuffered() {
    let mut buf = StreamingBuffer::new(BufferPolicy::Unbuffered);
    
    buf.push(AgentEvent::Token { content: "token".to_string() });
    let batch = buf.flush();
    
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_streaming_buffer_buffered() {
    let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 3 });
    
    let r1 = buf.push(AgentEvent::Token { content: "t1".to_string() });
    assert!(!r1);
    
    let r2 = buf.push(AgentEvent::Token { content: "t2".to_string() });
    assert!(!r2);
    
    let r3 = buf.push(AgentEvent::Token { content: "t3".to_string() });
    assert!(r3); // Should be ready at threshold
    
    let batch = buf.flush();
    assert_eq!(batch.len(), 3);
}

#[test]
fn test_streaming_buffer_tool_result_batching() {
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
    assert!(ready3);
    
    let batch = buf.flush();
    assert_eq!(batch.len(), 3);
}

