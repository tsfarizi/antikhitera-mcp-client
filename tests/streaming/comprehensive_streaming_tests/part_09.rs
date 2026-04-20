// ============================================================================
// SERIALIZATION & DESERIALIZATION
// ============================================================================

#[test]
fn test_streaming_request_serialization() {
    let request = StreamingRequest {
        mode: StreamingMode::Event,
        include_final_response: false,
        max_buffered_events: Some(100),
        phase2: None,
    };
    
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: StreamingRequest = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.mode, request.mode);
    assert_eq!(deserialized.include_final_response, request.include_final_response);
    assert_eq!(deserialized.max_buffered_events, request.max_buffered_events);
}

#[test]
fn test_agent_event_token_serialization() {
    let event = AgentEvent::Token { content: "hello".to_string() };
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: AgentEvent = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized, event);
}

