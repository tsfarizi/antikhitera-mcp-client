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
    stream.push_chunk("\u{4e16}\u{754c}");
    stream.push_chunk(" \u{1f680}");
    stream.complete();
    
    assert_eq!(stream.collect_all(), "Hello \u{4e16}\u{754c} \u{1f680}");
}

