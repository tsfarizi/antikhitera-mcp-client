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
//!
//! ## Usage
//!
//! ### Basic Logging (No Subscription)
//!
//! ```rust
//! use antikythera_log::{Logger, LogLevel, LogFilter};
//!
//! let logger = Logger::new("my-session");
//! logger.info("Agent started");
//! logger.debug("Processing LLM response");
//! logger.warn("Max steps approaching");
//! logger.error("Tool execution failed");
//!
//! // Fetch logs periodically
//! let logs = logger.get_logs(&LogFilter::new());
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
//! //     println!("[{}] {}", entry.level, entry.message);
//! // }
//!
//! // Logs are automatically sent to all subscribers
//! logger.info("This will be sent to subscriber in real-time");
//! ```

pub mod entries;
pub mod logger;

#[cfg(feature = "subscriber")]
pub mod subscriber;

pub mod session_logs;

pub use entries::*;
pub use logger::*;

#[cfg(feature = "subscriber")]
pub use subscriber::{LogSender, LogSubscriber};

pub use session_logs::{SessionLogExport, BatchLogExport};
