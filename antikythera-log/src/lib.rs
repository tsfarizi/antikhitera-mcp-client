//! # Antikythera Log
//!
//! Unified logging system for the Antikythera MCP Framework.
//!
//! ## Features
//!
//! - **Structured logging** - JSON-encoded log entries with context
//! - **Multiple levels** - Debug, Info, Warn, Error
//! - **Subscription system** - Real-time log streaming via subscribers
//! - **Periodic polling** - Fetch logs on-demand without subscription
//! - **WASM compatible** - Works in both native and WASM environments
//! - **Session tracking** - Logs grouped by session ID for traceability
//! - **Convenience macros** - `alog_info!`, `alog_debug!`, `alog_warn!`, `alog_error!`
//! - **Compile-time lint** - Blocks `println!`, `eprintln!`, `dbg!`, and `tracing` macros
//!
//! ## Usage
//!
//! ### Basic Logging (No Subscription)
//!
//! ```rust
//! use antikythera_log::{Logger, LogLevel, LogFilter, alog_info, alog_debug};
//!
//! let logger = Logger::new("my-session");
//! alog_info!(logger, "Agent started on port {}", 8080);
//! alog_debug!(logger, "Processing LLM response");
//! logger.warn("Max steps approaching");
//! logger.error("Tool execution failed");
//!
//! // Fetch logs periodically
//! let logs = logger.get_logs(&LogFilter::new());
//! ```
//!
//! ### With Source Module
//!
//! ```rust
//! use antikythera_log::{Logger, alog_info_src};
//!
//! let logger = Logger::new("my-session");
//! alog_info_src!(logger, "transport", "Connected to {}", "mcp-server");
//! ```
//!
//! ### Subscription (Real-time Streaming)
//!
//! ```rust,ignore
//! use antikythera_log::{Logger, LogSubscriber};
//!
//! let logger = Logger::new("my-session");
//!
//! // Create subscriber
//! let subscriber = logger.subscribe();
//!
//! // In another thread/task:
//! // while let Ok(entry) = subscriber.recv() {
//! //     cli_print!("[{}] {}", entry.level, entry.message);
//! // }
//!
//! // Logs are automatically sent to all subscribers
//! logger.info("This will be sent to subscriber in real-time");
//! ```
//!
//! ### Compile-Time Lint (Feature: `lint`)
//!
//! Enable the `lint` feature to block non-standard logging at compile time:
//!
//! ```toml
//! antikythera-log = { path = "../antikythera-log", features = ["lint"] }
//! ```
//!
//! This will produce compile errors for `println!`, `eprintln!`, `dbg!`,
//! and bare `tracing` macros. Use `cli_print!` / `cli_eprint!` for
//! legitimate CLI output.

// Macros must be declared before other modules so they are available
// to all downstream code.
#[macro_use]
pub mod macros;

pub mod entries;
pub mod logger;

#[cfg(feature = "subscriber")]
pub mod subscriber;

pub mod session_logs;

/// Compile-time lint module.
///
/// When the `lint` feature is enabled, importing this module's macros
/// will shadow `println!`, `eprintln!`, `dbg!`, and tracing macros
/// with versions that produce compile errors.
#[cfg(feature = "lint")]
pub mod lint;

pub use entries::*;
pub use logger::*;

#[cfg(feature = "subscriber")]
pub use subscriber::{LogSender, LogSubscriber};

pub use session_logs::{BatchLogExport, SessionLogExport};

/// Shared trait for Postcard binary serialization / deserialization.
///
/// Provides a default `to_postcard` / `from_postcard` pair backed by
/// [`postcard::to_allocvec`] and [`postcard::from_bytes`], unifying the
/// duplicated pattern across export types.
pub trait PostcardSerde: serde::Serialize + serde::de::DeserializeOwned + Sized {
    fn to_postcard(&self) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(self).map_err(|e| format!("Postcard serialize error: {e}"))
    }
    fn from_postcard(data: &[u8]) -> Result<Self, String> {
        postcard::from_bytes(data).map_err(|e| format!("Postcard deserialize error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_new_stores_session_id() {
        let logger = Logger::new("test-session");
        assert_eq!(logger.session_id(), "test-session");
    }

    #[test]
    fn logger_log_levels_record_correctly() {
        let logger = Logger::new("test");
        logger.debug("debug msg");
        logger.info("info msg");
        logger.warn("warn msg");
        logger.error("error msg");

        let logs = logger.get_logs(&LogFilter::new());
        assert_eq!(logs.total_count, 4);
        assert_eq!(logs.entries[0].level, LogLevel::Debug);
        assert_eq!(logs.entries[1].level, LogLevel::Info);
        assert_eq!(logs.entries[2].level, LogLevel::Warn);
        assert_eq!(logs.entries[3].level, LogLevel::Error);
    }

    #[test]
    fn log_buffer_evicts_oldest_on_overflow() {
        let logger = Logger::with_capacity("test", 3);
        logger.info("a");
        logger.info("b");
        logger.info("c");
        logger.info("d");

        let logs = logger.get_logs(&LogFilter::new());
        assert_eq!(logs.total_count, 3);
        assert_eq!(logs.entries[0].message, "b");
        assert_eq!(logs.entries[1].message, "c");
        assert_eq!(logs.entries[2].message, "d");
    }

    #[test]
    fn log_filter_by_level() {
        let logger = Logger::new("test");
        logger.debug("debug");
        logger.info("info");
        logger.error("error");

        let filter = LogFilter::new().min_level(LogLevel::Warn);
        let logs = logger.get_logs(&filter);
        assert_eq!(logs.total_count, 1);
        assert_eq!(logs.entries[0].level, LogLevel::Error);
    }

    #[test]
    fn log_filter_pagination() {
        let logger = Logger::new("test");
        for i in 0..10 {
            logger.info(&format!("msg-{i}"));
        }

        let filter = LogFilter::new().offset(3).limit(2);
        let batch = logger.get_logs(&filter);
        assert_eq!(batch.entries.len(), 2);
        assert_eq!(batch.entries[0].message, "msg-3");
        assert_eq!(batch.entries[1].message, "msg-4");
        assert!(batch.has_more);
    }

    #[test]
    fn logger_clone_shares_buffer() {
        let logger = Logger::new("test");
        let clone = logger.clone();

        logger.info("from-original");
        clone.info("from-clone");

        let logs = logger.get_logs(&LogFilter::new());
        assert_eq!(logs.total_count, 2);
    }

    #[test]
    fn log_with_source_and_context() {
        let logger = Logger::new("test");
        logger.log_with_source(LogLevel::Info, "agent", "agent started");
        logger.log_with_context(LogLevel::Debug, "processing", "{\"step\": 1}");

        let logs = logger.get_logs(&LogFilter::new());
        assert_eq!(logs.total_count, 2);
        assert_eq!(logs.entries[0].source.as_deref(), Some("agent"));
        assert_eq!(logs.entries[1].context.as_deref(), Some("{\"step\": 1}"));
    }

    #[test]
    fn log_level_from_str() {
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("Warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
    }

    #[test]
    fn log_buffer_clear() {
        let logger = Logger::new("test");
        logger.info("a");
        logger.info("b");
        assert_eq!(logger.len(), 2);

        logger.clear();
        assert_eq!(logger.len(), 0);
        assert!(logger.is_empty());
    }
}
