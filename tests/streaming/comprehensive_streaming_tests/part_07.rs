// ============================================================================
// EDGE CASES & BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_empty_token_content() {
    let event = AgentEvent::Token { content: "".to_string() };
    
    let mut stream = AgentEventStream::new();
    stream.push(event);
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_very_long_token_content() {
    let long_content = "x".repeat(10_000_000);
    let event = AgentEvent::Token { content: long_content };
    
    let mut stream = AgentEventStream::new();
    stream.push(event);
    
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_unicode_in_events() {
    let mut stream = AgentEventStream::new();
    
    stream.push_token("Hello \u{4e16}\u{754c} \u{0645}\u{0631}\u{062d}\u{0628}\u{0627} \u{1f680}");
    stream.push_tool("\u{83b7}\u{53d6}_\u{5929}\u{6c14}", ToolEventPhase::Started);
    stream.push_state("\u{5904}\u{7406}\u{4e2d}", Some("\u{7b49}\u{5f85}".to_string()));
    
    assert_eq!(stream.len(), 3);
}

#[test]
fn test_tool_result_with_empty_chunk() {
    let event = AgentEvent::ToolResult {
        tool_name: "grep".to_string(),
        chunk: "".to_string(),
        is_final: false,
    };
    
    let mut stream = AgentEventStream::new();
    stream.push(event);
    assert_eq!(stream.len(), 1);
}

#[test]
fn test_summary_with_zero_message_count() {
    let event = AgentEvent::Summary {
        chunk: "summary".to_string(),
        is_final: true,
        original_message_count: 0,
    };
    
    let mut stream = AgentEventStream::new();
    stream.push(event);
    assert_eq!(stream.len(), 1);
}

