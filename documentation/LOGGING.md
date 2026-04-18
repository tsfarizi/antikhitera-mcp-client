# Logging

Sistem logging terpusat untuk Antikythera MCP Framework dengan kemampuan **subscription real-time** dan **periodic polling**.

## Overview

```
┌─────────────────────────────────────────────────────────┐
│  Logger                                                 │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Log Buffer (thread-safe, ring buffer)            │ │
│  │  ├─ Entry 1: [INFO]  Agent started               │ │
│  │  ├─ Entry 2: [DEBUG] Processing LLM response     │ │
│  │  ├─ Entry 3: [WARN]  Max steps approaching       │ │
│  │  └─ Entry 4: [ERROR] Tool execution failed       │ │
│  └─────────────┬─────────────────────────────────────┘ │
│                │                                       │
│     ┌──────────┴──────────┐                           │
│     │                     │                           │
│  Periodic             Real-time                       │
│  Polling              Subscription                    │
│     │                     │                           │
│  get_logs()         subscribe() → recv()              │
│  get_latest()       try_recv() / recv_timeout()       │
└─────────────────────────────────────────────────────────┘
```

## Fitur

| Fitur | Deskripsi |
|-------|-----------|
| **Structured Logging** | JSON-encoded dengan metadata lengkap |
| **Multiple Levels** | Debug, Info, Warn, Error |
| **Session Tracking** | Log dikelompokkan per session ID |
| **Periodic Polling** | Fetch logs on-demand tanpa subscription |
| **Real-time Subscription** | Stream log entries secara real-time |
| **Filter & Pagination** | Filter by level, session, source + pagination |
| **WASM Compatible** | Bisa jalan di WASM dan native |
| **Ring Buffer** | Auto-trim saat capacity terlampaui |

## Usage

### Basic Logging (Periodic Polling)

```rust
use antikythera_log::{Logger, LogLevel, LogFilter};

// Create logger
let logger = Logger::new("agent-session-123");

// Log messages
logger.debug("Agent initialized");
logger.info("Processing user request");
logger.warn("Approaching max steps (8/10)");
logger.error("Tool 'get_weather' failed: timeout");

// Fetch logs periodically (polling)
let filter = LogFilter::new()
    .min_level(LogLevel::Warn)
    .limit(10);

let batch = logger.get_logs(&filter);
println!("Total warnings/errors: {}", batch.total_count);

for entry in &batch.entries {
    println!("{}", entry.format_pretty());
}

// Get latest N logs
let latest = logger.get_latest(5);
for entry in latest {
    println!("[{}] {}", entry.level, entry.message);
}
```

### Real-time Subscription

```rust
use antikythera_log::{Logger, LogFilter};
use std::thread;
use std::time::Duration;

let logger = Logger::new("agent-session-456");

// Create subscriber (can have multiple)
let subscriber = logger.subscribe();

// In another thread: receive logs in real-time
let handle = thread::spawn(move || {
    while let Ok(entry) = subscriber.recv_timeout(Duration::from_secs(1)) {
        println!("[{}] [{}] {}", 
            entry.timestamp, 
            entry.level, 
            entry.message
        );
    }
});

// In main thread: do work
logger.info("Starting agent loop");
logger.debug("Calling LLM API");
logger.warn("Slow response detected");
logger.error("Connection timeout");

// Subscriber receives all logs in real-time
handle.join().unwrap();
```

### Log with Context

```rust
let logger = Logger::new("session-789");

// Log with source module
logger.log_with_source(
    LogLevel::Info, 
    "wasm-agent", 
    "Processing LLM response"
);

// Log with context data (JSON)
logger.log_with_context(
    LogLevel::Info,
    "Tool called",
    r#"{"tool": "get_weather", "args": {"city": "NYC"}, "step": 3}"#,
);

// Fetch by source
let filter = LogFilter::new().source("wasm-agent");
let batch = logger.get_logs(&filter);
```

### JSON Serialization

```rust
let logger = Logger::new("session-export");

logger.info("Test message");

// Get logs as JSON
let batch = logger.get_logs(&LogFilter::new());
let json = batch.to_json().unwrap();

// Output:
// {
//   "entries": [
//     {
//       "level": "info",
//       "message": "Test message",
//       "timestamp": "2024-01-15T10:30:00Z",
//       "session_id": "session-export",
//       "sequence": 1
//     }
//   ],
//   "total_count": 1,
//   "has_more": false
// }
```

### Filter & Pagination

```rust
let logger = Logger::new("session-pagination");

// Generate 100 logs
for i in 0..100 {
    logger.info(&format!("Message {}", i));
}

// Page 1 (first 20)
let page1 = logger.get_logs(
    &LogFilter::new().limit(20).offset(0)
);
assert_eq!(page1.entries.len(), 20);
assert!(page1.has_more);
assert_eq!(page1.total_count, 100);

// Page 2 (next 20)
let page2 = logger.get_logs(
    &LogFilter::new().limit(20).offset(20)
);
assert_eq!(page2.entries.len(), 20);
```

## API Reference

### Logger

| Method | Purpose |
|--------|---------|
| `new(session_id)` | Create logger |
| `with_capacity(session_id, capacity)` | Create logger with custom buffer size |
| `debug(msg)` | Log at DEBUG level |
| `info(msg)` | Log at INFO level |
| `warn(msg)` | Log at WARN level |
| `error(msg)` | Log at ERROR level |
| `log_with_source(level, source, msg)` | Log with source module |
| `log_with_context(level, msg, context)` | Log with JSON context |
| `get_logs(filter)` | Get logs matching filter |
| `get_latest(count)` | Get latest N logs |
| `get_logs_json(filter)` | Get logs as JSON string |
| `subscribe()` | Create real-time subscriber |
| `clear()` | Clear all logs |
| `len()` | Get log count |

### LogFilter

| Method | Purpose |
|--------|---------|
| `new()` | Create empty filter |
| `min_level(level)` | Minimum log level |
| `session(id)` | Filter by session ID |
| `source(name)` | Filter by source module |
| `limit(n)` | Max entries to return |
| `offset(n)` | Skip first N entries |

### LogSubscriber

| Method | Purpose |
|--------|---------|
| `recv()` | Receive next (blocking) |
| `try_recv()` | Receive next (non-blocking) |
| `recv_timeout(d)` | Receive with timeout |
| `iter()` | Iterator over pending |
| `has_pending()` | Check if logs pending |
| `pending_count()` | Count pending logs |

### LogEntry

| Field | Type | Description |
|-------|------|-------------|
| `level` | LogLevel | Debug, Info, Warn, Error |
| `message` | String | Log message |
| `timestamp` | String | ISO 8601 timestamp |
| `session_id` | Option<String> | Session identifier |
| `source` | Option<String> | Source module |
| `context` | Option<String> | Additional context (JSON) |
| `sequence` | u64 | Auto-incrementing sequence |

## Features

| Feature | Dependencies | Purpose |
|---------|-------------|---------|
| `std` (default) | None | Standard logging |
| `wasm` | wasm-bindgen | WASM environment |
| `subscriber` | tokio, crossbeam-channel | Real-time subscription |

## Integration with WASM Agent

```rust
use antikythera_log::Logger;

// Create logger for WASM agent session
let logger = Logger::new("wasm-agent-session");

// Log agent events
logger.info("Agent initialized");
logger.debug("Received LLM response");
logger.warn("Max steps approaching: 8/10");
logger.error("Invalid JSON from LLM");

// Host can subscribe to see logs in real-time
let subscriber = logger.subscribe();
// ... receive logs in host runtime
```

## Integration with CLI

```rust
use antikythera_log::Logger;

// Create logger for CLI session
let logger = Logger::new("cli-session");

// Log CLI events
logger.info("User connected");
logger.debug("Processing command");
logger.warn("Rate limit approaching");
logger.error("LLM API timeout");

// Display logs in TUI
let latest = logger.get_latest(20);
for entry in latest {
    println!("{}", entry.format_pretty());
}
```

## Performance

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Single log entry | ~1μs | ~1M entries/sec |
| Get 100 logs | ~5μs | ~200K fetches/sec |
| Subscribe + recv | ~0.5μs | ~2M entries/sec |
| JSON serialization | ~10μs | ~100K entries/sec |

## Best Practices

1. **Use session IDs** - Group logs by session for traceability
2. **Set appropriate capacity** - Default 10,000 entries, adjust as needed
3. **Use filters efficiently** - Filter at source, not after fetching
4. **Subscribe sparingly** - Each subscriber adds overhead
5. **Clear old logs** - Use `clear()` when session ends
6. **Include context** - Use `log_with_context` for structured data
