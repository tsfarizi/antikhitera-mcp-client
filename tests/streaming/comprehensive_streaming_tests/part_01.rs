// ============================================================================
// STREAMING REQUEST TESTS
// ============================================================================

#[test]
fn test_streaming_request_default() {
    let req = StreamingRequest::default();
    
    assert_eq!(req.mode, StreamingMode::Mixed);
    assert!(req.include_final_response);
    assert_eq!(req.max_buffered_events, None);
}

#[test]
fn test_streaming_request_wants_tokens() {
    let token_mode = StreamingRequest {
        mode: StreamingMode::Token,
        ..Default::default()
    };
    assert!(token_mode.wants_tokens());
    
    let event_mode = StreamingRequest {
        mode: StreamingMode::Event,
        ..Default::default()
    };
    assert!(event_mode.wants_events());
    
    let mixed_mode = StreamingRequest {
        mode: StreamingMode::Mixed,
        ..Default::default()
    };
    assert!(mixed_mode.wants_tokens());
    assert!(mixed_mode.wants_events());
}

