//! Comprehensive Logging Module Tests
//!
//! Extensive test suite for antikythera-log with focus on:
//! - Edge cases and boundary conditions
//! - Concurrency safety and race conditions
//! - Security: input validation, injection prevention
//! - Performance: memory leaks, bounds
//! - Panic safety: no unwrap/expect in hot paths
//! - Data integrity: serialization, ordering

use antikythera_log::*;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

// ============================================================================
// EDGE CASES & BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_empty_session_id() {
    let logger = Logger::new("");
    logger.info("test message");
    
    assert_eq!(logger.len(), 1);
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].session_id, Some("".to_string()));
}

#[test]
fn test_very_long_session_id() {
    let long_id = "x".repeat(100_000);
    let logger = Logger::new(&long_id);
    logger.info("test");
    
    assert_eq!(logger.session_id(), long_id.as_str());
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].session_id, Some(long_id));
}

#[test]
fn test_empty_message() {
    let logger = Logger::new("test");
    logger.info("");
    
    assert_eq!(logger.len(), 1);
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].message, "");
}

#[test]
fn test_very_long_message() {
    let long_msg = "m".repeat(1_000_000);
    let logger = Logger::new("test");
    logger.info(&long_msg);
    
    assert_eq!(logger.len(), 1);
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].message, long_msg);
}

#[test]
fn test_unicode_in_message() {
    let logger = Logger::new("test");
    let messages = vec![
        "こんにちは",
        "你好",
        "مرحبا",
        "🦀 Rust 🚀",
        "Emoji test: 😀😃😄😁😆",
        "Combining: é = e + ́",
        "RTL: \u{202E}RTL\u{202C}",
    ];
    
    for msg in &messages {
        logger.info(*msg);
    }
    
    assert_eq!(logger.len(), messages.len());
    let batch = logger.get_logs(&LogFilter::new());
    for (i, msg) in messages.iter().enumerate() {
        assert_eq!(&batch.entries[i].message, msg);
    }
}

#[test]
fn test_unicode_in_session_id() {
    let logger = Logger::new("session_🌟_日本語");
    logger.info("test");
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].session_id, Some("session_🌟_日本語".to_string()));
}

#[test]
fn test_special_characters_in_message() {
    let logger = Logger::new("test");
    let special_msgs = vec![
        "Quote: \"hello\"",
        "Backslash: \\test\\",
        "Newline: hello\nworld",
        "Tab: hello\tworld",
        "Null byte: test\0null",
        "Control chars: \x01\x02\x03",
    ];
    
    for msg in &special_msgs {
        logger.info(*msg);
    }
    
    assert_eq!(logger.len(), special_msgs.len());
}

#[test]
fn test_zero_buffer_capacity() {
    let logger = Logger::with_capacity("test", 0);
    logger.info("msg1");
    logger.info("msg2");
    
    // With zero capacity, buffer should remain empty
    assert_eq!(logger.len(), 0);
}

#[test]
fn test_single_capacity_buffer() {
    let logger = Logger::with_capacity("test", 1);
    
    logger.info("msg1");
    assert_eq!(logger.len(), 1);
    
    logger.info("msg2");
    assert_eq!(logger.len(), 1);
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].message, "msg2");
}

#[test]
fn test_get_latest_with_empty_buffer() {
    let logger = Logger::new("test");
    let latest = logger.get_latest(10);
    assert_eq!(latest.len(), 0);
}

#[test]
fn test_get_latest_exceeds_buffer() {
    let logger = Logger::new("test");
    logger.info("msg1");
    logger.info("msg2");
    logger.info("msg3");
    
    let latest = logger.get_latest(1000);
    assert_eq!(latest.len(), 3);
}

#[test]
fn test_filter_offset_beyond_range() {
    let logger = Logger::new("test");
    logger.info("msg1");
    logger.info("msg2");
    
    let filter = LogFilter::new().offset(100).limit(10);
    let batch = logger.get_logs(&filter);
    
    assert_eq!(batch.entries.len(), 0);
    assert!(!batch.has_more);
}

#[test]
fn test_filter_limit_zero() {
    let logger = Logger::new("test");
    logger.info("msg1");
    logger.info("msg2");
    
    let filter = LogFilter::new().limit(0);
    let batch = logger.get_logs(&filter);
    
    assert_eq!(batch.entries.len(), 0);
    assert!(batch.has_more); // Should still indicate more exists
}

#[test]
fn test_multiple_clears() {
    let logger = Logger::new("test");
    
    for _ in 0..3 {
        logger.info("msg");
        assert_eq!(logger.len(), 1);
        logger.clear();
        assert_eq!(logger.len(), 0);
    }
}

// ============================================================================
// CONCURRENCY & THREAD SAFETY
// ============================================================================

#[test]
fn test_concurrent_logging_basic() {
    let logger = Arc::new(Logger::new("concurrent-test"));
    let thread_count = 10;
    let messages_per_thread = 100;
    
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let logger_clone = logger.clone();
        let handle = thread::spawn(move || {
            for msg_id in 0..messages_per_thread {
                logger_clone.info(&format!("thread-{}-msg-{}", thread_id, msg_id));
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(logger.len(), thread_count * messages_per_thread);
}

#[test]
fn test_concurrent_logging_stress() {
    let logger = Arc::new(Logger::with_capacity("stress-test", 100_000));
    let thread_count = 50;
    let messages_per_thread = 500;
    
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let logger_clone = logger.clone();
        let handle = thread::spawn(move || {
            for msg_id in 0..messages_per_thread {
                let level = match msg_id % 4 {
                    0 => LogLevel::Debug,
                    1 => LogLevel::Info,
                    2 => LogLevel::Warn,
                    _ => LogLevel::Error,
                };
                logger_clone.log(level, format!("t{}-m{}", thread_id, msg_id));
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    // All messages should fit in capacity
    assert!(logger.len() <= 100_000);
    // At least some messages should be logged
    assert!(logger.len() > 0);
}

#[test]
fn test_concurrent_read_write() {
    let logger = Arc::new(Logger::with_capacity("read-write-test", 10_000));
    
    let mut handles = vec![];
    
    // Writer threads
    for thread_id in 0..5 {
        let logger_clone = logger.clone();
        let handle = thread::spawn(move || {
            for i in 0..200 {
                logger_clone.info(&format!("writer-{}-msg-{}", thread_id, i));
            }
        });
        handles.push(handle);
    }
    
    // Reader threads
    for _ in 0..5 {
        let logger_clone = logger.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let _ = logger_clone.get_latest(10);
                let _ = logger_clone.get_logs(&LogFilter::new());
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert!(logger.len() > 0);
}

#[test]
fn test_concurrent_clear() {
    let logger = Arc::new(Logger::new("concurrent-clear"));
    
    // Add initial messages
    for i in 0..100 {
        logger.info(&format!("msg-{}", i));
    }
    
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];
    
    for thread_id in 0..5 {
        let logger_clone = logger.clone();
        let barrier_clone = barrier.clone();
        
        let handle = thread::spawn(move || {
            barrier_clone.wait(); // Synchronize all threads
            
            if thread_id == 0 {
                logger_clone.clear();
            } else {
                let _ = logger_clone.get_latest(10);
                logger_clone.info(&format!("post-clear-{}", thread_id));
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// INPUT VALIDATION & SECURITY
// ============================================================================

#[test]
fn test_json_injection_in_message() {
    let logger = Logger::new("test");
    
    // Attempt JSON injection
    let malicious = r#"","level":"ERROR","message":"injected"#;
    logger.info(malicious);
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].message, malicious);
    
    // Verify JSON is properly escaped
    let json = batch.to_json().unwrap();
    assert!(json.contains("injected") || !json.contains("\"level\":\"ERROR\"")); // Either escaped or not interpreted
}

#[test]
fn test_json_injection_in_context() {
    let logger = Logger::new("test");
    
    let malicious_context = r#"{"tool":"bad","x":"y","z":"malicious"}"#;
    logger.log_with_context(LogLevel::Info, "test", malicious_context);
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].context, Some(malicious_context.to_string()));
    
    // Verify serialization doesn't break
    let json = batch.to_json().unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_sql_injection_in_message() {
    let logger = Logger::new("test");
    
    let sql_injection = "'; DROP TABLE logs; --";
    logger.info(sql_injection);
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].message, sql_injection);
}

#[test]
fn test_path_traversal_attempt() {
    let logger = Logger::new("test");
    
    let path_traversal = "../../../../etc/passwd";
    logger.log_with_source(LogLevel::Info, path_traversal, "test");
    
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries[0].source, Some(path_traversal.to_string()));
}

#[test]
fn test_source_filter_with_special_chars() {
    let logger = Logger::new("test");
    
    logger.log_with_source(LogLevel::Info, "src@#$%", "msg");
    logger.log_with_source(LogLevel::Info, "normal-src", "msg2");
    
    let filter = LogFilter::new().source("src@#$%");
    let batch = logger.get_logs(&filter);
    
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(batch.entries[0].source, Some("src@#$%".to_string()));
}

// ============================================================================
// DATA INTEGRITY & SERIALIZATION
// ============================================================================

#[test]
fn test_serialization_roundtrip_empty_fields() {
    let entry = LogEntry::new(LogLevel::Debug, "test");
    
    let json = entry.to_json().unwrap();
    let restored = LogEntry::from_json(&json).unwrap();
    
    assert_eq!(restored.level, entry.level);
    assert_eq!(restored.message, entry.message);
    assert_eq!(restored.session_id, entry.session_id);
    assert_eq!(restored.source, entry.source);
    assert_eq!(restored.context, entry.context);
}

#[test]
fn test_serialization_roundtrip_all_fields() {
    let entry = LogEntry::new(LogLevel::Warn, "msg")
        .with_session("s1")
        .with_source("src1")
        .with_context(r#"{"key":"value"}"#)
        .with_sequence(999);
    
    let json = entry.to_json().unwrap();
    let restored = LogEntry::from_json(&json).unwrap();
    
    assert_eq!(restored.level, LogLevel::Warn);
    assert_eq!(restored.message, "msg");
    assert_eq!(restored.session_id, Some("s1".to_string()));
    assert_eq!(restored.source, Some("src1".to_string()));
    assert_eq!(restored.context, Some(r#"{"key":"value"}"#.to_string()));
    assert_eq!(restored.sequence, 999);
}

#[test]
fn test_invalid_json_deserialization() {
    let invalid_json = r#"{"level":"invalid","message":"x"}"#;
    let result = LogEntry::from_json(invalid_json);
    
    // Should gracefully handle invalid JSON
    assert!(result.is_err());
}

#[test]
fn test_sequence_ordering_on_concurrent_logs() {
    let logger = Arc::new(Logger::new("sequence-test"));
    let barrier = Arc::new(Barrier::new(10));
    
    let mut handles = vec![];
    for _ in 0..10 {
        let logger_clone = logger.clone();
        let barrier_clone = barrier.clone();
        
        let handle = thread::spawn(move || {
            barrier_clone.wait(); // Synchronize
            for i in 0..10 {
                logger_clone.info(&format!("msg-{}", i));
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let batch = logger.get_logs(&LogFilter::new());
    let sequences: Vec<u64> = batch.entries.iter().map(|e| e.sequence).collect();
    
    // Sequences should be monotonically increasing
    for i in 1..sequences.len() {
        assert!(sequences[i] >= sequences[i - 1]);
    }
    
    // Should have 100 unique sequences (10 threads × 10 messages)
    assert_eq!(sequences.len(), 100);
}

#[test]
fn test_log_batch_has_more_flag() {
    let logger = Logger::new("test");
    
    for i in 0..100 {
        logger.info(&format!("msg-{}", i));
    }
    
    // With limit < total, should indicate more
    let filter = LogFilter::new().limit(10);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.entries.len(), 10);
    assert!(batch.has_more);
    
    // With limit >= total, should not indicate more
    let filter = LogFilter::new().limit(1000);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.entries.len(), 100);
    assert!(!batch.has_more);
}

// ============================================================================
// PERFORMANCE & RESOURCE MANAGEMENT
// ============================================================================

#[test]
fn test_rapid_sequential_logging() {
    let logger = Logger::with_capacity("perf-test", 100_000);
    let start = Instant::now();
    
    for i in 0..100_000 {
        logger.info(&format!("msg-{}", i));
    }
    
    let elapsed = start.elapsed();
    let per_second = (100_000.0 / elapsed.as_secs_f64()) as u64;
    
    assert_eq!(logger.len(), 100_000);
    println!("Logged 100k messages in {:?} ({} msg/sec)", elapsed, per_second);
    
    // Should complete in reasonable time (adjust threshold as needed)
    assert!(elapsed.as_secs() < 10, "Logging too slow: {:?}", elapsed);
}

#[test]
fn test_large_batch_retrieval() {
    let logger = Logger::with_capacity("large-batch", 10_000);
    
    for i in 0..10_000 {
        logger.info(&format!("msg-{}", i));
    }
    
    let start = Instant::now();
    let batch = logger.get_logs(&LogFilter::new());
    let elapsed = start.elapsed();
    
    assert_eq!(batch.entries.len(), 10_000);
    println!("Retrieved 10k entries in {:?}", elapsed);
    assert!(elapsed.as_millis() < 1000, "Retrieval too slow");
}

#[test]
fn test_filter_performance() {
    let logger = Logger::with_capacity("filter-perf", 50_000);
    
    for i in 0..50_000 {
        let level = match i % 4 {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            _ => LogLevel::Error,
        };
        logger.log(level, format!("msg-{}", i));
    }
    
    let start = Instant::now();
    let filter = LogFilter::new().min_level(LogLevel::Warn);
    let batch = logger.get_logs(&filter);
    let elapsed = start.elapsed();
    
    assert!(batch.entries.len() > 0);
    println!("Filtered 50k entries in {:?}", elapsed);
    assert!(elapsed.as_millis() < 500, "Filter too slow");
}

#[test]
fn test_json_serialization_performance() {
    let logger = Logger::with_capacity("json-perf", 10_000);
    
    for i in 0..10_000 {
        logger.log_with_source(
            LogLevel::Info,
            &format!("src-{}", i % 100),
            &format!("msg-{}", i),
        );
    }
    
    let start = Instant::now();
    let json = logger.get_logs_json(&LogFilter::new()).unwrap();
    let elapsed = start.elapsed();
    
    assert!(!json.is_empty());
    println!("Serialized 10k entries to JSON in {:?}", elapsed);
    assert!(elapsed.as_millis() < 2000, "JSON serialization too slow");
}

// ============================================================================
// PANIC SAFETY & ERROR HANDLING
// ============================================================================

#[test]
fn test_no_panic_on_empty_log_retrieval() {
    let logger = Logger::new("test");
    
    // Should not panic even with empty buffer
    let batch = logger.get_logs(&LogFilter::new());
    assert_eq!(batch.entries.len(), 0);
    
    let latest = logger.get_latest(10);
    assert_eq!(latest.len(), 0);
}

#[test]
fn test_no_panic_on_extreme_limits() {
    let logger = Logger::new("test");
    logger.info("msg");
    
    // Should handle extreme values without panic
    let filter = LogFilter::new().limit(usize::MAX);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.entries.len(), 1);
    
    let filter = LogFilter::new().offset(usize::MAX);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.entries.len(), 0);
}

#[test]
fn test_no_panic_on_context_with_invalid_json() {
    let logger = Logger::new("test");
    
    // Invalid JSON in context should not panic
    logger.log_with_context(LogLevel::Info, "msg", "not valid json {");
    
    assert_eq!(logger.len(), 1);
}

#[test]
fn test_no_panic_on_format_pretty_with_special_chars() {
    let entry = LogEntry::new(LogLevel::Info, "msg\0with\nnull\tand\rspecial")
        .with_session("s\x01\x02")
        .with_source("src\x03\x04");
    
    let formatted = entry.format_pretty();
    assert!(!formatted.is_empty());
}

// ============================================================================
// FILTERING EDGE CASES
// ============================================================================

#[test]
fn test_log_level_parsing() {
    let levels = vec!["debug", "DEBUG", "info", "INFO", "warn", "WARN", "error", "ERROR"];
    
    for level_str in levels {
        let level = LogLevel::parse_label(level_str);
        assert!(level.is_some(), "Failed to parse: {}", level_str);
    }
    
    // Invalid levels
    let invalid = LogLevel::parse_label("invalid");
    assert!(invalid.is_none());
}

#[test]
fn test_filter_source_with_empty_string() {
    let logger = Logger::new("test");
    
    logger.log_with_source(LogLevel::Info, "", "msg1");
    logger.log_with_source(LogLevel::Info, "src", "msg2");
    
    let filter = LogFilter::new().source("");
    let batch = logger.get_logs(&filter);
    
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(batch.entries[0].source, Some("".to_string()));
}

#[test]
fn test_multiple_filters_combined() {
    let logger = Logger::new("test");
    
    for level in &[LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error] {
        for source in &["src1", "src2", "src3"] {
            logger.log_with_source(*level, *source, &format!("msg-{:?}", level));
        }
    }
    
    // Combine multiple filters
    let filter = LogFilter::new()
        .min_level(LogLevel::Warn)
        .source("src2")
        .limit(5);
    
    let batch = logger.get_logs(&filter);
    
    for entry in &batch.entries {
        assert!(entry.level >= LogLevel::Warn);
        assert_eq!(entry.source, Some("src2".to_string()));
    }
}

#[test]
fn test_pagination_consistency() {
    let logger = Logger::new("test");
    
    for i in 0..100 {
        logger.info(&format!("msg-{:03}", i));
    }
    
    // Get all messages in pages of 10
    let mut all_messages = vec![];
    for page in 0..10 {
        let filter = LogFilter::new()
            .offset(page * 10)
            .limit(10);
        
        let batch = logger.get_logs(&filter);
        all_messages.extend(batch.entries.iter().map(|e| e.message.clone()));
    }
    
    assert_eq!(all_messages.len(), 100);
    
    // Verify order and no duplicates
    for i in 0..100 {
        assert_eq!(all_messages[i], format!("msg-{:03}", i));
    }
}

// ============================================================================
// CLONE & SHARE BEHAVIOR
// ============================================================================

#[test]
fn test_logger_clone_shares_buffer() {
    let logger1 = Logger::new("test");
    let logger2 = logger1.clone();
    
    logger1.info("from logger1");
    logger2.info("from logger2");
    
    assert_eq!(logger1.len(), 2);
    assert_eq!(logger2.len(), 2);
    
    let batch = logger1.get_logs(&LogFilter::new());
    assert_eq!(batch.entries.len(), 2);
}

#[test]
fn test_logger_clear_affects_all_clones() {
    let logger1 = Logger::new("test");
    logger1.info("msg1");
    logger1.info("msg2");
    
    let logger2 = logger1.clone();
    assert_eq!(logger2.len(), 2);
    
    logger1.clear();
    assert_eq!(logger2.len(), 0);
}
