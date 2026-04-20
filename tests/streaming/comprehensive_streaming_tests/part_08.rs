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

