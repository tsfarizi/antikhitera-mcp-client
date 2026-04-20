//! Comprehensive Streaming Module Tests
//!
//! Extensive test suite for antikythera-core streaming with focus on:
//! - Buffer capacity and overflow handling
//! - Event ordering and consistency
//! - Panic safety under load
//! - Edge cases and boundary conditions
//! - Concurrent event generation
//! - Performance under pressure

use antikythera_core::{
    AgentEvent, AgentEventStream, BufferPolicy, ClientInputStream, StreamingBuffer,
    StreamingMode, StreamingRequest, ToolEventPhase,
};
use std::sync::Arc;
use std::thread;

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

// ============================================================================
// BUFFER CAPACITY & OVERFLOW
// ============================================================================

#[test]
fn test_agent_event_stream_enforces_capacity_bound() {
    let mut stream = AgentEventStream::with_max_buffered_events(Some(5));
    
    // Add more than capacity
    for i in 0..10 {
        stream.push_token(&format!("token-{}", i));
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
        stream.push_token(&format!("token-{}", i));
    }
    
    assert_eq!(stream.len(), 100_000);
}

// ============================================================================
// CLIENT INPUT STREAM TESTS
// ============================================================================

#[test]
fn test_client_input_stream_basic() {
    let mut stream = ClientInputStream::new();
    stream.push_chunk("hello");
    stream.complete();
    
    assert!(stream.is_complete());
    assert_eq!(stream.collect_all(), "hello");
}

#[test]
fn test_client_input_stream_multiple_chunks() {
    let mut stream = ClientInputStream::new();
    
    stream.push_chunk("hello");
    stream.push_chunk(" ");
    stream.push_chunk("world");
    stream.complete();
    
    assert_eq!(stream.collect_all(), "hello world");
}

#[test]
fn test_client_input_stream_large_input() {
    let mut stream = ClientInputStream::new();
    let large_input = "x".repeat(1_000_000);
    
    stream.push_chunk(&large_input);
    stream.complete();
    
    assert_eq!(stream.collect_all(), large_input);
}

#[test]
fn test_client_input_stream_unicode() {
    let mut stream = ClientInputStream::new();
    
    stream.push_chunk("Hello ");
    stream.push_chunk("世界");
    stream.push_chunk(" 🚀");
    stream.complete();
    
    assert_eq!(stream.collect_all(), "Hello 世界 🚀");
}

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
    
    stream.push_token("Hello 世界 مرحبا 🚀");
    stream.push_tool("获取_天气", ToolEventPhase::Started);
    stream.push_state("处理中", Some("等待".to_string()));
    
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

// ============================================================================
// CONCURRENT EVENT GENERATION
// ============================================================================

#[test]
fn test_concurrent_token_generation() {
    let stream = Arc::new(std::sync::Mutex::new(AgentEventStream::new()));
    let thread_count = 10;
    let tokens_per_thread = 100;
    
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let stream_clone = stream.clone();
        let handle = thread::spawn(move || {
            for token_id in 0..tokens_per_thread {
                let mut s = stream_clone.lock().unwrap();
                s.push_token(format!("t{}-{}", thread_id, token_id));
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_stream = stream.lock().unwrap();
    assert_eq!(final_stream.len(), thread_count * tokens_per_thread);
}

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
