//! SDK Logging Module
//!
//! Unified logging for all SDK operations with automatic source module tracking.
//! Captures all FFI interactions and SDK functionality.

use antikythera_log::{LogBatch, LogEntry, LogFilter, LogLevel, Logger};
use std::sync::{Arc, LazyLock, Mutex};

// ============================================================================
/// Global SDK Logger Registry
// ============================================================================

/// Global logger storage for SDK
static SDK_LOGGERS: LazyLock<Mutex<std::collections::HashMap<String, Arc<Logger>>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Get or create a logger for an SDK session
pub fn get_sdk_logger(session_id: &str) -> Arc<Logger> {
    let mut loggers = SDK_LOGGERS.lock().unwrap();

    loggers
        .entry(session_id.to_string())
        .or_insert_with(|| Arc::new(Logger::new(session_id)))
        .clone()
}

/// Clear all SDK loggers
pub fn clear_sdk_loggers() {
    let mut loggers = SDK_LOGGERS.lock().unwrap();
    loggers.clear();
}

// ============================================================================
/// Module-Specific Loggers (with automatic source tracking)
// ============================================================================

/// Config FFI module logger
pub struct ConfigFfiLogger {
    logger: Arc<Logger>,
}

impl ConfigFfiLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn ffi_call(&self, function: &str, args: &str) {
        let context = format!("{{\"function\": \"{}\", \"args\": {}}}", function, args);
        self.logger
            .log_with_context(LogLevel::Debug, format!("FFI call: {}", function), context);
    }

    pub fn ffi_result(&self, function: &str, success: bool, result_size: usize) {
        let context = format!(
            "{{\"function\": \"{}\", \"success\": {}, \"result_size\": {}}}",
            function, success, result_size
        );
        self.logger.log_with_context(
            LogLevel::Debug,
            format!("FFI result: {}", function),
            context,
        );
    }

    pub fn ffi_error(&self, function: &str, error: &str) {
        let context = format!(
            "{{\"function\": \"{}\", \"error\": \"{}\"}}",
            function, error
        );
        self.logger
            .log_with_context(LogLevel::Error, format!("FFI error: {}", function), context);
    }

    pub fn config_loaded(&self, source: &str, size: usize) {
        let context = format!("{{\"source\": \"{}\", \"size\": {}}}", source, size);
        self.logger
            .log_with_context(LogLevel::Info, "Config loaded", context);
    }

    pub fn config_saved(&self, path: &str, size: usize) {
        let context = format!("{{\"path\": \"{}\", \"size\": {}}}", path, size);
        self.logger
            .log_with_context(LogLevel::Info, "Config saved", context);
    }

    pub fn provider_added(&self, provider_id: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Provider added",
            format!("{{\"provider_id\": \"{}\"}}", provider_id),
        );
    }

    pub fn provider_removed(&self, provider_id: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Provider removed",
            format!("{{\"provider_id\": \"{}\"}}", provider_id),
        );
    }

    pub fn prompt_updated(&self, prompt_name: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Prompt updated",
            format!("{{\"prompt_name\": \"{}\"}}", prompt_name),
        );
    }

    pub fn agent_config_changed(&self, field: &str, value: &str) {
        let context = format!("{{\"field\": \"{}\", \"value\": \"{}\"}}", field, value);
        self.logger
            .log_with_context(LogLevel::Info, "Agent config changed", context);
    }
}

/// Server management logger
pub struct ServerLogger {
    logger: Arc<Logger>,
}

impl ServerLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn server_created(&self, server_id: u32, addr: &str) {
        let context = format!("{{\"server_id\": {}, \"addr\": \"{}\"}}", server_id, addr);
        self.logger
            .log_with_context(LogLevel::Info, "Server created", context);
    }

    pub fn server_stopped(&self, server_id: u32) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Server stopped",
            format!("{{\"server_id\": {}}}", server_id),
        );
    }

    pub fn server_listed(&self, count: usize) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Server listed",
            format!("{{\"count\": {}}}", count),
        );
    }

    pub fn server_error(&self, server_id: u32, error: &str) {
        let context = format!("{{\"server_id\": {}, \"error\": \"{}\"}}", server_id, error);
        self.logger
            .log_with_context(LogLevel::Error, "Server error", context);
    }
}

/// Agent management logger
pub struct AgentLogger {
    logger: Arc<Logger>,
}

impl AgentLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn agent_registered(&self, agent_id: &str, agent_type: &str) {
        let context = format!(
            "{{\"agent_id\": \"{}\", \"type\": \"{}\"}}",
            agent_id, agent_type
        );
        self.logger
            .log_with_context(LogLevel::Info, "Agent registered", context);
    }

    pub fn agent_unregistered(&self, agent_id: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Agent unregistered",
            format!("{{\"agent_id\": \"{}\"}}", agent_id),
        );
    }

    pub fn agent_listed(&self, count: usize) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Agent listed",
            format!("{{\"count\": {}}}", count),
        );
    }

    pub fn agent_status_checked(&self, agent_id: &str) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Agent status checked",
            format!("{{\"agent_id\": \"{}\"}}", agent_id),
        );
    }
}

/// Prompt management logger
pub struct PromptLogger {
    logger: Arc<Logger>,
}

impl PromptLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn prompt_get(&self, prompt_name: &str) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Prompt retrieved",
            format!("{{\"prompt_name\": \"{}\"}}", prompt_name),
        );
    }

    pub fn prompt_set(&self, prompt_name: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Prompt updated",
            format!("{{\"prompt_name\": \"{}\"}}", prompt_name),
        );
    }

    pub fn prompt_listed(&self, count: usize) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Prompts listed",
            format!("{{\"count\": {}}}", count),
        );
    }
}

/// Response formatting logger
pub struct ResponseLogger {
    logger: Arc<Logger>,
}

impl ResponseLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn format_set(&self, server_id: u32, format_is_json: bool) {
        let context = format!(
            "{{\"server_id\": {}, \"format_is_json\": {}}}",
            server_id, format_is_json
        );
        self.logger
            .log_with_context(LogLevel::Info, "Response format set", context);
    }

    pub fn response_formatted(&self, server_id: u32, output_size: usize) {
        let context = format!(
            "{{\"server_id\": {}, \"output_size\": {}}}",
            server_id, output_size
        );
        self.logger
            .log_with_context(LogLevel::Debug, "Response formatted", context);
    }
}

/// WASM Agent processing logger
pub struct WasmAgentLogger {
    logger: Arc<Logger>,
}

impl WasmAgentLogger {
    pub fn new(session_id: &str) -> Self {
        Self {
            logger: get_sdk_logger(session_id),
        }
    }

    pub fn llm_response_processed(&self, session_id: &str, action: &str) {
        let context = format!(
            "{{\"session_id\": \"{}\", \"action\": \"{}\"}}",
            session_id, action
        );
        self.logger
            .log_with_context(LogLevel::Debug, "LLM response processed", context);
    }

    pub fn tool_result_processed(&self, session_id: &str, tool_name: &str) {
        let context = format!(
            "{{\"session_id\": \"{}\", \"tool\": \"{}\"}}",
            session_id, tool_name
        );
        self.logger
            .log_with_context(LogLevel::Debug, "Tool result processed", context);
    }

    pub fn state_retrieved(&self, session_id: &str) {
        self.logger.log_with_context(
            LogLevel::Debug,
            "Agent state retrieved",
            format!("{{\"session_id\": \"{}\"}}", session_id),
        );
    }

    pub fn session_reset(&self, session_id: &str) {
        self.logger.log_with_context(
            LogLevel::Info,
            "Agent session reset",
            format!("{{\"session_id\": \"{}\"}}", session_id),
        );
    }
}

// ============================================================================
/// SDK Log Query API
// ============================================================================

/// Query SDK logs with filter
pub fn query_sdk_logs(session_id: &str, filter: &LogFilter) -> LogBatch {
    if let Some(logger) = SDK_LOGGERS.lock().unwrap().get(session_id) {
        logger.get_logs(filter)
    } else {
        LogBatch::new(Vec::new(), 0, false)
    }
}

/// Get latest SDK logs
pub fn get_latest_sdk_logs(session_id: &str, count: usize) -> Vec<LogEntry> {
    if let Some(logger) = SDK_LOGGERS.lock().unwrap().get(session_id) {
        logger.get_latest(count)
    } else {
        Vec::new()
    }
}

/// Get SDK logs as JSON
pub fn get_sdk_logs_json(session_id: &str, filter: &LogFilter) -> Result<String, String> {
    if let Some(logger) = SDK_LOGGERS.lock().unwrap().get(session_id) {
        logger.get_logs_json(filter)
    } else {
        Ok(r#"{"entries":[],"total_count":0,"has_more":false}"#.to_string())
    }
}

/// Subscribe to real-time SDK log stream
pub fn subscribe_sdk_logs(session_id: &str) -> Option<antikythera_log::LogSubscriber> {
    SDK_LOGGERS
        .lock()
        .unwrap()
        .get(session_id)
        .map(|l| l.subscribe())
}

/// Clear SDK logs
pub fn clear_sdk_session_logs(session_id: &str) {
    if let Some(logger) = SDK_LOGGERS.lock().unwrap().get(session_id) {
        logger.clear();
    }
}
