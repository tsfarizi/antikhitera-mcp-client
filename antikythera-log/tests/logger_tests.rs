//! Logger Tests

use antikythera_log::*;

#[test]
fn test_basic_logging() {
    let logger = Logger::new("test-session");

    logger.debug("Debug message");
    logger.info("Info message");
    logger.warn("Warn message");
    logger.error("Error message");

    assert_eq!(logger.len(), 4);
}

#[test]
fn test_log_levels() {
    let logger = Logger::new("test-levels");

    logger.debug("debug");
    logger.info("info");
    logger.warn("warn");
    logger.error("error");

    let latest = logger.get_latest(1);
    assert_eq!(latest[0].level, LogLevel::Error);
    assert_eq!(latest[0].message, "error");
}

#[test]
fn test_log_filter() {
    let logger = Logger::new("test-filter");

    logger.debug("debug1");
    logger.info("info1");
    logger.warn("warn1");
    logger.error("error1");
    logger.debug("debug2");

    // Filter by min level
    let filter = LogFilter::new().min_level(LogLevel::Warn);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.total_count, 2);
    assert_eq!(batch.entries.len(), 2);

    // Filter by limit
    let filter = LogFilter::new().limit(2);
    let batch = logger.get_logs(&filter);
    assert_eq!(batch.entries.len(), 2);
    assert!(batch.has_more);
}

#[test]
fn test_log_json() {
    let logger = Logger::new("test-json");

    logger.info("Test message");

    let batch = logger.get_logs(&LogFilter::new());
    let json = batch.to_json().unwrap();

    assert!(json.contains("Test message"));
    assert!(json.contains("info")); // lowercase due to serde rename
}

#[test]
fn test_log_with_source() {
    let logger = Logger::new("test-source");

    logger.log_with_source(LogLevel::Info, "wasm-agent", "Agent started");

    let filter = LogFilter::new().source("wasm-agent");
    let batch = logger.get_logs(&filter);

    assert_eq!(batch.total_count, 1);
    assert_eq!(batch.entries[0].source, Some("wasm-agent".to_string()));
}

#[test]
fn test_log_with_context() {
    let logger = Logger::new("test-context");

    logger.log_with_context(
        LogLevel::Info,
        "Tool called",
        r#"{"tool": "get_weather", "args": {"city": "NYC"}}"#,
    );

    let latest = logger.get_latest(1);
    assert!(latest[0].context.is_some());
    assert!(latest[0].context.as_ref().unwrap().contains("get_weather"));
}

#[test]
fn test_clear_logs() {
    let logger = Logger::new("test-clear");

    logger.info("msg1");
    logger.info("msg2");
    assert_eq!(logger.len(), 2);

    logger.clear();
    assert_eq!(logger.len(), 0);
}

#[test]
fn test_session_id() {
    let logger = Logger::new("my-session");
    assert_eq!(logger.session_id(), "my-session");

    logger.info("test");

    let latest = logger.get_latest(1);
    assert_eq!(latest[0].session_id, Some("my-session".to_string()));
}

#[test]
fn test_log_entry_serialization() {
    let entry = LogEntry::new(LogLevel::Info, "Test")
        .with_session("session-1")
        .with_source("wasm-agent")
        .with_sequence(42);

    let json = entry.to_json().unwrap();
    let restored = LogEntry::from_json(&json).unwrap();

    assert_eq!(restored.level, LogLevel::Info);
    assert_eq!(restored.message, "Test");
    assert_eq!(restored.session_id, Some("session-1".to_string()));
    assert_eq!(restored.sequence, 42);
}

#[test]
fn test_log_buffer_capacity() {
    let logger = Logger::with_capacity("test-capacity", 5);

    for i in 0..10 {
        logger.info(&format!("msg-{}", i));
    }

    assert_eq!(logger.len(), 5);

    let latest = logger.get_latest(5);
    assert_eq!(latest[0].message, "msg-5");
    assert_eq!(latest[4].message, "msg-9");
}
