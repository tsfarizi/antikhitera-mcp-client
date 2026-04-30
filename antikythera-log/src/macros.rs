//! # Logging Macros
//!
//! Convenient macros for the Antikythera logging system.
//! These are the **only** sanctioned way to emit log messages in the framework.
//!
//! ## Usage
//!
//! ```rust
//! use antikythera_log::{Logger, alog_info, alog_debug, alog_warn, alog_error};
//!
//! let logger = Logger::new("my-session");
//! alog_info!(logger, "Server started on port {}", 8080);
//! alog_debug!(logger, "Processing request");
//! alog_warn!(logger, "Connection pool low: {} remaining", 2);
//! alog_error!(logger, "Failed to connect: {}", "timeout");
//! ```
//!
//! ### With source module
//!
//! ```rust
//! use antikythera_log::{Logger, alog_info_src};
//!
//! let logger = Logger::new("my-session");
//! alog_info_src!(logger, "transport", "Connected to server {}", "mcp-1");
//! ```
//!
//! ### With context
//!
//! ```rust
//! use antikythera_log::{Logger, alog_info_ctx};
//!
//! let logger = Logger::new("my-session");
//! alog_info_ctx!(logger, "Tool call completed", r#"{"tool": "read_file"}"#);
//! ```

/// Log at DEBUG level.
///
/// # Examples
/// ```rust
/// use antikythera_log::{Logger, alog_debug};
/// let logger = Logger::new("test");
/// alog_debug!(logger, "value = {}", 42);
/// ```
#[macro_export]
macro_rules! alog_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.debug(format!($($arg)*))
    };
}

/// Log at INFO level.
///
/// # Examples
/// ```rust
/// use antikythera_log::{Logger, alog_info};
/// let logger = Logger::new("test");
/// alog_info!(logger, "started");
/// ```
#[macro_export]
macro_rules! alog_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info(format!($($arg)*))
    };
}

/// Log at WARN level.
///
/// # Examples
/// ```rust
/// use antikythera_log::{Logger, alog_warn};
/// let logger = Logger::new("test");
/// alog_warn!(logger, "low memory: {}MB", 128);
/// ```
#[macro_export]
macro_rules! alog_warn {
    ($logger:expr, $($arg:tt)*) => {
        $logger.warn(format!($($arg)*))
    };
}

/// Log at ERROR level.
///
/// # Examples
/// ```rust
/// use antikythera_log::{Logger, alog_error};
/// let logger = Logger::new("test");
/// alog_error!(logger, "connection failed: {}", "timeout");
/// ```
#[macro_export]
macro_rules! alog_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error(format!($($arg)*))
    };
}

/// Log at DEBUG level with source module tag.
#[macro_export]
macro_rules! alog_debug_src {
    ($logger:expr, $source:expr, $($arg:tt)*) => {
        $logger.log_with_source($crate::LogLevel::Debug, $source, format!($($arg)*))
    };
}

/// Log at INFO level with source module tag.
#[macro_export]
macro_rules! alog_info_src {
    ($logger:expr, $source:expr, $($arg:tt)*) => {
        $logger.log_with_source($crate::LogLevel::Info, $source, format!($($arg)*))
    };
}

/// Log at WARN level with source module tag.
#[macro_export]
macro_rules! alog_warn_src {
    ($logger:expr, $source:expr, $($arg:tt)*) => {
        $logger.log_with_source($crate::LogLevel::Warn, $source, format!($($arg)*))
    };
}

/// Log at ERROR level with source module tag.
#[macro_export]
macro_rules! alog_error_src {
    ($logger:expr, $source:expr, $($arg:tt)*) => {
        $logger.log_with_source($crate::LogLevel::Error, $source, format!($($arg)*))
    };
}

/// Log at DEBUG level with context.
#[macro_export]
macro_rules! alog_debug_ctx {
    ($logger:expr, $msg:expr, $ctx:expr) => {
        $logger.log_with_context($crate::LogLevel::Debug, $msg, $ctx)
    };
}

/// Log at INFO level with context.
#[macro_export]
macro_rules! alog_info_ctx {
    ($logger:expr, $msg:expr, $ctx:expr) => {
        $logger.log_with_context($crate::LogLevel::Info, $msg, $ctx)
    };
}

/// Log at WARN level with context.
#[macro_export]
macro_rules! alog_warn_ctx {
    ($logger:expr, $msg:expr, $ctx:expr) => {
        $logger.log_with_context($crate::LogLevel::Warn, $msg, $ctx)
    };
}

/// Log at ERROR level with context.
#[macro_export]
macro_rules! alog_error_ctx {
    ($logger:expr, $msg:expr, $ctx:expr) => {
        $logger.log_with_context($crate::LogLevel::Error, $msg, $ctx)
    };
}

/// Sanctioned replacement for `println!` in CLI binaries.
///
/// Use this ONLY for user-facing CLI output (help text, config wizard, etc.).
#[macro_export]
macro_rules! cli_print {
    ($($arg:tt)*) => {
        std::println!($($arg)*)
    };
}

/// Sanctioned replacement for `eprintln!` in CLI binaries.
///
/// Use this ONLY for user-facing CLI error output.
#[macro_export]
macro_rules! cli_eprint {
    ($($arg:tt)*) => {
        std::eprintln!($($arg)*)
    };
}
