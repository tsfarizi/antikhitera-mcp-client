//! Core Logging Module
//!
//! Centralized logging for antikythera-core.
//! All log entries automatically include the source module.

use antikythera_log::{LogBatch, LogEntry, LogFilter, LogLevel, Logger};
use std::sync::{Arc, LazyLock};

// ============================================================================
// Global Logger Registry
// ============================================================================

/// Global logger storage
static LOGGERS: LazyLock<std::sync::Mutex<std::collections::HashMap<String, Arc<Logger>>>> =
    LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Active session key used by the TUI tracing bridge.
/// Defaults to "tui" so all tracing events land in a predictable bucket.
static ACTIVE_SESSION: LazyLock<std::sync::Mutex<String>> =
    LazyLock::new(|| std::sync::Mutex::new("tui".to_string()));

/// Set the session key that the TUI tracing bridge writes events to.
/// Call this from the TUI when a session becomes active.
pub fn set_active_session(session_id: &str) {
    let mut s = ACTIVE_SESSION.lock().unwrap();
    *s = session_id.to_string();
}

/// Get the current active session key used by the TUI tracing bridge.
pub fn get_active_session() -> String {
    ACTIVE_SESSION.lock().unwrap().clone()
}

/// Get or create a logger for a session
pub fn get_logger(session_id: &str) -> Arc<Logger> {
    let mut loggers = LOGGERS.lock().unwrap();

    loggers
        .entry(session_id.to_string())
        .or_insert_with(|| Arc::new(Logger::new(session_id)))
        .clone()
}

/// Clear all loggers
pub fn clear_all_loggers() {
    let mut loggers = LOGGERS.lock().unwrap();
    loggers.clear();
}

/// Get logger count
pub fn logger_count() -> usize {
    let loggers = LOGGERS.lock().unwrap();
    loggers.len()
}

// ============================================================================
// Module Loggers (with automatic source tracking)
// ============================================================================

/// Config module logger
pub struct ConfigLogger {
    logger: Arc<Logger>,
}

impl ConfigLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_logger(session_id),
        }
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Debug, "config", message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Info, "config", message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Warn, "config", message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Error, "config", message);
    }
}

/// Agent module logger
pub struct AgentLogger {
    logger: Arc<Logger>,
}

impl AgentLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_logger(session_id),
        }
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Debug, "agent", message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Info, "agent", message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Warn, "agent", message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Error, "agent", message);
    }

    pub fn tool_call(&self, tool: &str, args: &serde_json::Value) {
        let context = format!("{{\"tool\": \"{}\", \"args\": {}}}", tool, args);
        self.logger
            .log_with_context(LogLevel::Info, format!("Tool call: {}", tool), context)
    }

    pub fn tool_result(&self, tool: &str, success: bool, step: u32) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Error
        };
        let context = format!(
            "{{\"tool\": \"{}\", \"success\": {}, \"step\": {}}}",
            tool, success, step
        );
        self.logger.log_with_context(
            level,
            format!("Tool result: {} (step {})", tool, step),
            context,
        )
    }

    pub fn agent_step(&self, step: u32, max_steps: u32) {
        self.logger.log_with_source(
            LogLevel::Debug,
            "agent",
            format!("Agent step {}/{}", step, max_steps),
        )
    }

    pub fn agent_complete(&self, steps: u32) {
        self.logger.log_with_source(
            LogLevel::Info,
            "agent",
            format!("Agent completed in {} steps", steps),
        )
    }
}

/// Transport module logger
pub struct TransportLogger {
    logger: Arc<Logger>,
}

impl TransportLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_logger(session_id),
        }
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Debug, "transport", message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Info, "transport", message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Warn, "transport", message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Error, "transport", message);
    }

    pub fn connect(&self, server: &str) {
        self.logger.log_with_source(
            LogLevel::Info,
            "transport",
            format!("Connecting to: {}", server),
        );
    }

    pub fn disconnect(&self, server: &str) {
        self.logger.log_with_source(
            LogLevel::Info,
            "transport",
            format!("Disconnected from: {}", server),
        );
    }

    pub fn tool_request(&self, server: &str, tool: &str) {
        self.logger.log_with_source(
            LogLevel::Debug,
            "transport",
            format!("Tool request: {}.{} ", server, tool),
        );
    }

    pub fn tool_response(&self, server: &str, tool: &str, success: bool) {
        let level = if success {
            LogLevel::Debug
        } else {
            LogLevel::Error
        };
        self.logger.log_with_source(
            level,
            "transport",
            format!("Tool response: {}.{} (success: {})", server, tool, success),
        );
    }
}

/// Provider module logger
pub struct ProviderLogger {
    logger: Arc<Logger>,
}

impl ProviderLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_logger(session_id),
        }
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Debug, "provider", message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Info, "provider", message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Warn, "provider", message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.logger
            .log_with_source(LogLevel::Error, "provider", message);
    }

    pub fn api_call(&self, provider: &str, model: &str) {
        self.logger.log_with_source(
            LogLevel::Debug,
            "provider",
            format!("API call: {} ({})", provider, model),
        );
    }

    pub fn api_response(&self, provider: &str, model: &str, tokens: Option<u32>) {
        let token_info = tokens
            .map(|t| format!(", {} tokens", t))
            .unwrap_or_default();
        self.logger.log_with_source(
            LogLevel::Debug,
            "provider",
            format!("API response: {} ({}){}", provider, model, token_info),
        );
    }

    pub fn api_error(&self, provider: &str, error: &str) {
        self.logger.log_with_source(
            LogLevel::Error,
            "provider",
            format!("API error: {} ({})", provider, error),
        );
    }
}

// ============================================================================
// Log Query API
// ============================================================================

/// Query logs across all modules
pub fn query_logs(session_id: &str, filter: &LogFilter) -> LogBatch {
    if let Some(logger) = LOGGERS.lock().unwrap().get(session_id) {
        logger.get_logs(filter)
    } else {
        LogBatch::new(Vec::new(), 0, false)
    }
}

/// Get latest logs across all modules
pub fn get_latest_logs(session_id: &str, count: usize) -> Vec<LogEntry> {
    if let Some(logger) = LOGGERS.lock().unwrap().get(session_id) {
        logger.get_latest(count)
    } else {
        Vec::new()
    }
}

/// Get logs as JSON
pub fn get_logs_json(session_id: &str, filter: &LogFilter) -> Result<String, String> {
    if let Some(logger) = LOGGERS.lock().unwrap().get(session_id) {
        logger.get_logs_json(filter)
    } else {
        Ok(r#"{"entries":[],"total_count":0,"has_more":false}"#.to_string())
    }
}

/// Subscribe to real-time log stream
pub fn subscribe_logs(session_id: &str) -> Option<antikythera_log::LogSubscriber> {
    LOGGERS
        .lock()
        .unwrap()
        .get(session_id)
        .map(|l| l.subscribe())
}

/// Clear logs for a session
pub fn clear_logs(session_id: &str) {
    if let Some(logger) = LOGGERS.lock().unwrap().get(session_id) {
        logger.clear();
    }
}
