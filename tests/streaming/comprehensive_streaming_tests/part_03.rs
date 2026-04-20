// ============================================================================
// AGENT EVENT STREAM TESTS
// ============================================================================

#[test]
fn test_agent_event_stream_default() {
    let stream = AgentEventStream::default();
    assert!(stream.is_empty());
}

#[test]
fn test_agent_event_stream_push_token() {
    let mut stream = AgentEventStream::new();
    stream.push_token("hello");
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_agent_event_stream_push_tool() {
    let mut stream = AgentEventStream::new();
    stream.push_tool("grep", ToolEventPhase::Started);
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_agent_event_stream_push_state() {
    let mut stream = AgentEventStream::new();
    stream.push_state("ready", Some("waiting".to_string()));
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_agent_event_stream_push_completed() {
    let mut stream = AgentEventStream::new();
    stream.push(AgentEvent::Completed);
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_agent_event_stream_multiple_events() {
    let mut stream = AgentEventStream::new();
    
    stream.push_token("token1");
    stream.push_token("token2");
    stream.push_tool("grep", ToolEventPhase::Started);
    stream.push_tool("grep", ToolEventPhase::Finished);
    stream.push(AgentEvent::Completed);
    
    assert_eq!(stream.len(), 5);
}

#[test]
fn test_agent_event_stream_drain() {
    let mut stream = AgentEventStream::new();
    
    stream.push_token("token1");
    stream.push_token("token2");
    
    let events = stream.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(stream.len(), 0);
}

#[test]
fn test_agent_event_stream_pop_next() {
    let mut stream = AgentEventStream::new();
    
    stream.push_token("token1");
    stream.push_token("token2");
    
    let first = stream.pop_next();
    assert!(first.is_some());
    assert_eq!(stream.len(), 1);
    
    let second = stream.pop_next();
    assert!(second.is_some());
    assert_eq!(stream.len(), 0);
}

#[test]
fn test_agent_event_stream_complete() {
    let mut stream = AgentEventStream::new();
    
    stream.push_token("test");
    stream.complete();
    
    assert_eq!(stream.len(), 2);
}

