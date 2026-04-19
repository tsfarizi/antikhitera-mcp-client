//! Agent Management Feature Slice
//!
//! This module provides types, registry, validation, and FFI bindings
//! for managing multi-agent configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, LazyLock};

// ============================================================================
// Types
// ============================================================================

/// Agent type/role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    GeneralAssistant,
    CodeReviewer,
    DataAnalyst,
    Researcher,
    Custom,
}

/// Agent skill level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Expert,
}

// ============================================================================
// Orchestrator hardening — SDK surface for manipulation and monitoring
//
// All fields are optional with sensible defaults so callers that do not need
// these controls can ignore them entirely.  No behaviour changes for existing
// code that does not opt in.
// ============================================================================

/// SDK-level orchestrator configuration options.
///
/// Pass this (optionally) when constructing a [`MultiAgentOrchestrator`] through
/// the SDK to control concurrency limits, step budgets, and retry behaviour.
///
/// ## Defaults
/// All fields are `None` / `Always` — meaning unlimited resources and retry on
/// every failure, which preserves the original behaviour.
///
/// ## Example (JSON)
/// ```json
/// {
///   "max_concurrent_tasks": 4,
///   "max_total_steps": 200,
///   "max_total_tasks": 50,
///   "default_retry_condition": "on_transient"
/// }
/// ```
///
/// [`MultiAgentOrchestrator`]: antikythera_core::application::agent::multi_agent::MultiAgentOrchestrator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestratorOptions {
    /// Maximum number of tasks that may execute concurrently (None = unlimited).
    #[serde(default)]
    pub max_concurrent_tasks: Option<usize>,
    /// Global step budget across all dispatched tasks in a session (None = unlimited).
    #[serde(default)]
    pub max_total_steps: Option<usize>,
    /// Maximum number of tasks that may be dispatched in a session (None = unlimited).
    #[serde(default)]
    pub max_total_tasks: Option<usize>,
    /// Default retry condition for tasks that do not specify their own policy.
    ///
    /// Accepted values: `"always"` (default), `"on_transient"`, `"never"`.
    #[serde(default)]
    pub default_retry_condition: RetryConditionOption,
}

/// SDK-friendly mirror of [`RetryCondition`] with `Default = Always`.
///
/// [`RetryCondition`]: antikythera_core::application::agent::multi_agent::RetryCondition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetryConditionOption {
    /// Retry on every failure (default).
    #[default]
    Always,
    /// Only retry when the error is classified as transient.
    OnTransient,
    /// Never retry, regardless of `max_retries`.
    Never,
}

/// Point-in-time monitoring snapshot for a running orchestrator.
///
/// Returned by [`mcp_orchestrator_snapshot`] and can be polled at any time to
/// inspect resource consumption without interrupting execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestratorMonitorSnapshot {
    /// Steps consumed so far across all dispatched tasks.
    pub consumed_steps: usize,
    /// Tasks dispatched so far (including in-flight ones).
    pub dispatched_tasks: usize,
    /// Configured step ceiling (`None` = unlimited).
    pub max_total_steps: Option<usize>,
    /// Configured task ceiling (`None` = unlimited).
    pub max_total_tasks: Option<usize>,
    /// Configured concurrency ceiling (`None` = unlimited).
    pub max_concurrent_tasks: Option<usize>,
    /// Whether the step budget has been exhausted.
    pub step_budget_exhausted: bool,
    /// Whether the task budget has been exhausted.
    pub task_budget_exhausted: bool,
    /// Whether the orchestrator has been externally cancelled.
    pub cancelled: bool,
}

/// Per-task introspection detail extracted from a [`TaskResult`].
///
/// Call [`mcp_task_result_detail`] with a serialized `TaskResult` JSON to
/// decode this without needing a live orchestrator reference.
///
/// [`TaskResult`]: antikythera_core::application::agent::multi_agent::TaskResult
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskResultDetail {
    /// Classified error kind (snake_case string) when the task failed, or `null`.
    pub error_kind: Option<String>,
    /// Whether the failure is transient (safe to retry).
    pub is_transient: bool,
    /// Name of the router that selected the agent (e.g. `"round-robin"`).
    pub router_name: Option<String>,
    /// Agent ID that was selected to handle the task.
    pub selected_agent_id: Option<String>,
    /// Number of candidate agents the router evaluated.
    pub candidates_considered: Option<usize>,
    /// Human-readable explanation of why the router chose this agent.
    pub routing_reason: Option<String>,
    /// Milliseconds the task waited for a concurrency slot before starting.
    pub concurrency_wait_ms: u64,
    /// Whether the task was rejected due to an exhausted budget.
    pub budget_exhausted: bool,
}

// ============================================================================
// Conversions between SDK types and core types
// ============================================================================

#[cfg(feature = "multi-agent")]
mod core_conversions {
    use super::*;
    use antikythera_core::application::agent::multi_agent::{
        OrchestratorBudget, BudgetSnapshot, RetryCondition, TaskResult,
    };

    impl From<&OrchestratorOptions> for OrchestratorBudget {
        /// Build an [`OrchestratorBudget`] from SDK options.
        ///
        /// Fields that are `None` result in unlimited resources (the default).
        fn from(opts: &OrchestratorOptions) -> Self {
            let mut budget = OrchestratorBudget::new();
            if let Some(steps) = opts.max_total_steps {
                budget = budget.with_max_total_steps(steps);
            }
            if let Some(tasks) = opts.max_total_tasks {
                budget = budget.with_max_total_tasks(tasks);
            }
            if let Some(concurrency) = opts.max_concurrent_tasks {
                budget = budget.with_max_concurrent_tasks(concurrency);
            }
            budget
        }
    }

    impl From<RetryConditionOption> for RetryCondition {
        fn from(opt: RetryConditionOption) -> Self {
            match opt {
                RetryConditionOption::Always => RetryCondition::Always,
                RetryConditionOption::OnTransient => RetryCondition::OnTransient,
                RetryConditionOption::Never => RetryCondition::Never,
            }
        }
    }

    impl From<&BudgetSnapshot> for OrchestratorMonitorSnapshot {
        fn from(snap: &BudgetSnapshot) -> Self {
            Self {
                consumed_steps: snap.consumed_steps,
                dispatched_tasks: snap.dispatched_tasks,
                max_total_steps: snap.max_total_steps,
                max_total_tasks: snap.max_total_tasks,
                max_concurrent_tasks: snap.max_concurrent_tasks,
                    step_budget_exhausted: snap.max_total_steps
                        .is_some_and(|max| snap.consumed_steps >= max),
                    task_budget_exhausted: snap.max_total_tasks
                        .is_some_and(|max| snap.dispatched_tasks >= max),
                cancelled: false, // caller merges cancellation state separately
            }
        }
    }

    impl OrchestratorMonitorSnapshot {
        /// Merge live cancellation state into an existing snapshot.
        pub fn with_cancelled(mut self, cancelled: bool) -> Self {
            self.cancelled = cancelled;
            self
        }
    }

    impl From<&TaskResult> for TaskResultDetail {
        fn from(result: &TaskResult) -> Self {
            let routing = result.metadata.routing_decision.as_ref();
            Self {
                error_kind: result
                    .error_kind
                    .as_ref()
                    .map(|k| serde_json::to_value(k).ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| format!("{:?}", k))),
                is_transient: result.is_transient(),
                router_name: routing.map(|r| r.router_name.clone()),
                selected_agent_id: routing.map(|r| r.selected_agent_id.clone()),
                candidates_considered: routing.map(|r| r.candidates_considered),
                routing_reason: routing.and_then(|r| r.reason.clone()),
                concurrency_wait_ms: result.metadata.concurrency_wait_ms,
                budget_exhausted: result.metadata.budget_exhausted,
            }
        }
    }
}

// ============================================================================
// FFI — Orchestrator options, monitoring, and task introspection
// ============================================================================

/// Return the default [`OrchestratorOptions`] as a JSON string.
///
/// Use this to obtain the canonical default configuration, then modify fields
/// as needed before passing to `mcp_build_orchestrator_budget`.
#[unsafe(no_mangle)]
pub extern "C" fn mcp_default_orchestrator_options() -> *mut c_char {
    serialize_result(&OrchestratorOptions::default())
}

/// Validate an [`OrchestratorOptions`] JSON string.
///
/// Returns `{"valid": true}` or `{"valid": false, "error": "..."}`.
#[unsafe(no_mangle)]
pub extern "C" fn mcp_validate_orchestrator_options(options_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(options_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };
    match serde_json::from_str::<OrchestratorOptions>(&json_str) {
        Ok(opts) => {
            // Basic sanity checks
            let mut errors: Vec<String> = Vec::new();
            if opts.max_concurrent_tasks == Some(0) {
                errors.push("max_concurrent_tasks must be > 0 if set".to_string());
            }
            if opts.max_total_steps == Some(0) {
                errors.push("max_total_steps must be > 0 if set".to_string());
            }
            if opts.max_total_tasks == Some(0) {
                errors.push("max_total_tasks must be > 0 if set".to_string());
            }
            if errors.is_empty() {
                to_c_string(r#"{"valid":true}"#)
            } else {
                serialize_result(&serde_json::json!({"valid": false, "errors": errors}))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    }
}

/// Decode a serialized [`TaskResult`] JSON into a [`TaskResultDetail`] JSON
/// for easy routing/error introspection without requiring a live orchestrator.
///
/// Returns `{"error": "..."}` if the input cannot be parsed.
#[cfg(feature = "multi-agent")]
#[unsafe(no_mangle)]
pub extern "C" fn mcp_task_result_detail(task_result_json: *const c_char) -> *mut c_char {
    use antikythera_core::application::agent::multi_agent::TaskResult;

    let json_str = match from_c_string(task_result_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error":"{}"}}"#, e)),
    };
    match serde_json::from_str::<TaskResult>(&json_str) {
        Ok(result) => serialize_result(&TaskResultDetail::from(&result)),
        Err(e) => to_c_string(&format!(r#"{{"error":"Invalid TaskResult JSON: {}"}}"#, e)),
    }
}

/// Build an [`OrchestratorMonitorSnapshot`] from a [`BudgetSnapshot`] JSON
/// (obtained via `MultiAgentOrchestrator::budget_snapshot()`) and an optional
/// `cancelled` boolean.
///
/// This is a pure decode helper — it performs no I/O.
#[cfg(feature = "multi-agent")]
#[unsafe(no_mangle)]
pub extern "C" fn mcp_orchestrator_snapshot(
    budget_snapshot_json: *const c_char,
    cancelled: bool,
) -> *mut c_char {
    use antikythera_core::application::agent::multi_agent::BudgetSnapshot;

    let json_str = match from_c_string(budget_snapshot_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error":"{}"}}"#, e)),
    };
    match serde_json::from_str::<BudgetSnapshot>(&json_str) {
        Ok(snap) => {
            let monitor = OrchestratorMonitorSnapshot::from(&snap).with_cancelled(cancelled);
            serialize_result(&monitor)
        }
        Err(e) => to_c_string(&format!(r#"{{"error":"Invalid BudgetSnapshot JSON: {}"}}"#, e)),
    }
}

/// Agent capability descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Capability name
    pub name: String,
    /// Skill level for this capability
    pub level: SkillLevel,
    /// Description of capability
    pub description: String,
}

/// Agent configuration with strict validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub id: String,
    /// Agent type/role
    #[serde(rename = "agent-type")]
    pub agent_type: AgentType,
    /// Display name
    pub name: String,
    /// Agent description
    pub description: Option<String>,
    /// Model provider to use
    pub model_provider: String,
    /// Model name to use
    pub model: String,
    /// Maximum steps allowed
    pub max_steps: u32,
    /// Whether agent can call tools
    pub can_call_tools: bool,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Custom system prompt (overrides default)
    pub custom_prompt: Option<String>,
    /// Temperature for LLM
    pub temperature: Option<f32>,
    /// Whether agent is enabled
    pub enabled: bool,
}

/// Agent validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentValidationResult {
    /// Whether configuration is valid
    pub valid: bool,
    /// List of validation errors
    pub errors: Vec<String>,
    /// Agent ID that was validated
    pub agent_id: String,
}

/// Agent status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID
    pub id: String,
    /// Agent name
    pub name: String,
    /// Whether agent is currently active
    pub active: bool,
    /// Current session ID (if active)
    pub session_id: Option<String>,
    /// Number of tasks completed
    pub tasks_completed: u32,
    /// Number of tasks failed
    pub tasks_failed: u32,
}

/// Agent task request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskRequest {
    /// Task description/prompt
    pub task: String,
    /// Optional session ID for continuity
    pub session_id: Option<String>,
    /// Maximum steps for this task
    pub max_steps: Option<u32>,
    /// Whether to allow tool calls
    pub allow_tools: Option<bool>,
}

/// Agent task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskResult {
    /// Task output
    pub response: String,
    /// Whether task succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Number of steps executed
    pub steps_executed: u32,
    /// Tools called during task
    pub tools_called: Vec<String>,
    /// Session ID (if any)
    pub session_id: Option<String>,
}

/// Multi-agent orchestration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    /// Final synthesized response
    pub response: String,
    /// Whether orchestration succeeded
    pub success: bool,
    /// Agent contributions (agent_id -> contribution)
    pub contributions: Vec<(String, String)>,
    /// Total steps across all agents
    pub total_steps: u32,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

// ============================================================================
// Validation
// ============================================================================

impl AgentConfig {
    /// Validate agent configuration
    pub fn validate(&self) -> AgentValidationResult {
        let mut errors = Vec::new();

        // ID validation
        if self.id.is_empty() {
            errors.push("Agent ID cannot be empty".to_string());
        }
        if !self.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push("Agent ID can only contain alphanumeric characters, hyphens, and underscores".to_string());
        }

        // Name validation
        if self.name.is_empty() {
            errors.push("Agent name cannot be empty".to_string());
        }

        // Model validation
        if self.model_provider.is_empty() {
            errors.push("Model provider cannot be empty".to_string());
        }
        if self.model.is_empty() {
            errors.push("Model name cannot be empty".to_string());
        }

        // Max steps validation
        if self.max_steps == 0 {
            errors.push("Max steps must be greater than 0".to_string());
        }

        // Temperature validation
        if let Some(temp) = self.temperature {
            if temp < 0.0 || temp > 2.0 {
                errors.push("Temperature must be between 0.0 and 2.0".to_string());
            }
        }

        AgentValidationResult {
            valid: errors.is_empty(),
            errors,
            agent_id: self.id.clone(),
        }
    }
}

// ============================================================================
// Registry
// ============================================================================

/// Agent registry
static AGENTS: LazyLock<Mutex<HashMap<String, AgentConfig>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Agent status tracking
static AGENT_STATUS: LazyLock<Mutex<HashMap<String, AgentStatus>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get mutable access to agent registry (for tests)
#[allow(dead_code)]
pub fn agents_lock() -> std::sync::MutexGuard<'static, HashMap<String, AgentConfig>> {
    AGENTS.lock().unwrap()
}

/// Get mutable access to agent status registry (for tests)
#[allow(dead_code)]
pub fn agent_status_lock() -> std::sync::MutexGuard<'static, HashMap<String, AgentStatus>> {
    AGENT_STATUS.lock().unwrap()
}

// ============================================================================
// FFI Bindings
// ============================================================================

fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn from_c_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Null pointer".to_string());
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

fn serialize_result<T: serde::Serialize>(result: &T) -> *mut c_char {
    match serde_json::to_string(result) {
        Ok(json) => to_c_string(&json),
        Err(e) => {
            let error = serde_json::json!({"error": format!("Serialization failed: {}", e)});
            to_c_string(&error.to_string())
        }
    }
}

/// Register a new agent configuration
#[unsafe(no_mangle)]
pub extern "C" fn mcp_register_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let validation = config.validate();
    if !validation.valid {
        return serialize_result(&validation);
    }

    let id = config.id.clone();
    match AGENTS.lock() {
        Ok(mut agents) => {
            if agents.contains_key(&id) {
                return serialize_result(&AgentValidationResult {
                    valid: false,
                    errors: vec![format!("Agent '{}' already exists", id)],
                    agent_id: id,
                });
            }

            let status = AgentStatus {
                id: id.clone(),
                name: config.name.clone(),
                active: false,
                session_id: None,
                tasks_completed: 0,
                tasks_failed: 0,
            };

            AGENT_STATUS.lock().unwrap().insert(id.clone(), status);
            agents.insert(id.clone(), config);

            serialize_result(&AgentValidationResult {
                valid: true,
                errors: vec![],
                agent_id: id,
            })
        }
        Err(e) => serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Failed to lock registry: {}", e)],
            agent_id: String::new(),
        }),
    }
}

/// Unregister an agent by ID
#[unsafe(no_mangle)]
pub extern "C" fn mcp_unregister_agent(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match AGENTS.lock() {
        Ok(mut agents) => {
            if agents.remove(&id_str).is_some() {
                AGENT_STATUS.lock().unwrap().remove(&id_str);
                to_c_string(r#"{"success": true}"#)
            } else {
                to_c_string(&format!(r#"{{"error": "Agent '{}' not found"}}"#, id_str))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// List all registered agents
#[unsafe(no_mangle)]
pub extern "C" fn mcp_list_agents() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get configuration for a specific agent
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_agent(id: *const c_char) -> *mut c_char {
    let id_str = match from_c_string(id) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    match AGENTS.lock() {
        Ok(agents) => {
            if let Some(config) = agents.get(&id_str) {
                serialize_result(config)
            } else {
                to_c_string(&format!(r#"{{"error": "Agent '{}' not found"}}"#, id_str))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get runtime status of all agents
#[unsafe(no_mangle)]
pub extern "C" fn mcp_get_agent_status() -> *mut c_char {
    match AGENT_STATUS.lock() {
        Ok(statuses) => {
            let status_list: Vec<&AgentStatus> = statuses.values().collect();
            serialize_result(&status_list)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Validate agent configuration without registering
#[unsafe(no_mangle)]
pub extern "C" fn mcp_validate_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return serialize_result(&AgentValidationResult {
            valid: false,
            errors: vec![format!("Invalid JSON: {}", e)],
            agent_id: String::new(),
        }),
    };

    serialize_result(&config.validate())
}

/// Export all agents configuration as JSON
#[unsafe(no_mangle)]
pub extern "C" fn mcp_export_agents_config() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Import agents configuration from JSON
#[unsafe(no_mangle)]
pub extern "C" fn mcp_import_agents_config(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    };

    let configs: Vec<AgentConfig> = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return to_c_string(&format!(r#"{{"error": "Invalid JSON: {}"}}"#, e)),
    };

    let count = configs.len();
    match AGENTS.lock() {
        Ok(mut agents) => {
            for config in configs {
                let id = config.id.clone();
                let status = AgentStatus {
                    id: id.clone(),
                    name: config.name.clone(),
                    active: false,
                    session_id: None,
                    tasks_completed: 0,
                    tasks_failed: 0,
                };

                AGENT_STATUS.lock().unwrap().insert(id.clone(), status);
                agents.insert(id, config);
            }
            to_c_string(&format!(r#"{{"success": true, "imported": {}}}"#, count))
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}
