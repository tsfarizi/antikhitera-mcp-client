// ============================================================================
// BUFFER CAPACITY & OVERFLOW
// ============================================================================

#[test]
fn test_agent_event_stream_enforces_capacity_bound() {
    let mut stream = AgentEventStream::with_max_buffered_events(Some(5));
    
    // Add more than capacity
    for i in 0..10 {
        stream.push_token(format!("token-{}", i));
    }
    
    // Should not exceed capacity
    assert!(stream.len() <= 5);
}

#[test]
fn test_agent_event_stream_zero_capacity() {
    let mut stream = AgentEventStream::with_max_buffered_events(Some(0));
    stream.push_token("token");
    
    // With zero capacity, events should be dropped
    assert_eq!(stream.len(), 0);
}

#[test]
fn test_agent_event_stream_single_capacity() {
    let mut stream = AgentEventStream::with_max_buffered_events(Some(1));
    
    stream.push_token("first");
    assert_eq!(stream.len(), 1);
    
    stream.push_token("second");
    assert_eq!(stream.len(), 1); // Only latest remains
}

#[test]
fn test_agent_event_stream_very_large_capacity() {
    let mut stream = AgentEventStream::with_max_buffered_events(Some(1_000_000));
    
    for i in 0..100_000 {
        stream.push_token(format!("token-{}", i));
    }
    
    assert_eq!(stream.len(), 100_000);
}

