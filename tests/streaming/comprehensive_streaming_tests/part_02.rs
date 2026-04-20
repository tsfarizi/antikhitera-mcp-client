// ============================================================================
// AGENT EVENT TESTS
// ============================================================================

#[test]
fn test_agent_event_token() {
    let event = AgentEvent::Token {
        content: "hello".to_string(),
    };
    
    if let AgentEvent::Token { content } = event {
        assert_eq!(content, "hello");
    } else {
        panic!("Expected Token event");
    }
}

#[test]
fn test_agent_event_tool() {
    let event = AgentEvent::Tool {
        tool_name: "grep".to_string(),
        phase: ToolEventPhase::Started,
    };
    
    if let AgentEvent::Tool { tool_name, phase } = event {
        assert_eq!(tool_name, "grep");
        assert_eq!(phase, ToolEventPhase::Started);
    } else {
        panic!("Expected Tool event");
    }
}

#[test]
fn test_agent_event_state() {
    let event = AgentEvent::State {
        state: "processing".to_string(),
        detail: Some("step 1".to_string()),
    };
    
    if let AgentEvent::State { state, detail } = event {
        assert_eq!(state, "processing");
        assert_eq!(detail, Some("step 1".to_string()));
    } else {
        panic!("Expected State event");
    }
}

#[test]
fn test_agent_event_tool_result() {
    let event = AgentEvent::ToolResult {
        tool_name: "grep".to_string(),
        chunk: "result line 1".to_string(),
        is_final: false,
    };
    
    if let AgentEvent::ToolResult { tool_name, chunk, is_final } = event {
        assert_eq!(tool_name, "grep");
        assert_eq!(chunk, "result line 1");
        assert!(!is_final);
    } else {
        panic!("Expected ToolResult event");
    }
}

#[test]
fn test_agent_event_summary() {
    let event = AgentEvent::Summary {
        chunk: "Summary chunk".to_string(),
        is_final: true,
        original_message_count: 42,
    };
    
    if let AgentEvent::Summary { chunk, is_final, original_message_count } = event {
        assert_eq!(chunk, "Summary chunk");
        assert!(is_final);
        assert_eq!(original_message_count, 42);
    } else {
        panic!("Expected Summary event");
    }
}

#[test]
fn test_agent_event_completed() {
    let event = AgentEvent::Completed;
    assert_eq!(event, AgentEvent::Completed);
}

