// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_rapid_event_generation() {
    let mut stream = AgentEventStream::new();
    
    for i in 0..10_000 {
        stream.push_token(format!("token-{}", i));
    }
    
    assert_eq!(stream.len(), 10_000);
}

#[test]
fn test_large_batch_flush() {
    let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 10_000 });
    
    for i in 0..10_000 {
        buf.push(AgentEvent::Token { content: format!("token-{}", i) });
    }
    
    let batch = buf.flush();
    assert_eq!(batch.len(), 10_000);
}
