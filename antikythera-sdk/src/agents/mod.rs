//! Agent Management Feature Slice
//!
//! This module provides types, registry, validation, and FFI bindings
//! for managing multi-agent configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

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

/// SDK-level streaming mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamingModeOption {
    Token,
    Event,
    #[default]
    Mixed,
}

/// Host-facing streaming options for incremental output.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamingOptions {
    #[serde(default)]
    pub mode: StreamingModeOption,
    #[serde(default = "default_true")]
    pub include_final_response: bool,
    #[serde(default)]
    pub max_buffered_events: Option<usize>,
}

const fn default_true() -> bool {
    true
}

impl StreamingOptions {
    /// Validate user-provided streaming options.
    pub fn validate(&self) -> Result<(), String> {
        let errors = validate_streaming_options_collect(self);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Convert SDK streaming options into core streaming request.
    #[cfg(feature = "sdk-core")]
    pub fn to_streaming_request(&self) -> antikythera_core::StreamingRequest {
        antikythera_core::StreamingRequest {
            mode: self.mode.into(),
            include_final_response: self.include_final_response,
            max_buffered_events: self.max_buffered_events,
            phase2: None,
        }
    }
}

#[cfg(feature = "sdk-core")]
impl From<StreamingModeOption> for antikythera_core::StreamingMode {
    fn from(value: StreamingModeOption) -> Self {
        match value {
            StreamingModeOption::Token => antikythera_core::StreamingMode::Token,
            StreamingModeOption::Event => antikythera_core::StreamingMode::Event,
            StreamingModeOption::Mixed => antikythera_core::StreamingMode::Mixed,
        }
    }
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
    /// Optional guardrail chain configuration exposed to host code as JSON.
    #[serde(default)]
    pub guardrails: GuardrailOptions,
}

/// Host-friendly guardrail configuration for the multi-agent orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardrailOptions {
    #[serde(default)]
    pub timeout: Option<TimeoutGuardrailOptions>,
    #[serde(default)]
    pub budget: Option<BudgetGuardrailOptions>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitGuardrailOptions>,
    #[serde(default)]
    pub cancellation: bool,
}

impl GuardrailOptions {
    /// Returns `true` when no guardrail config is enabled.
    pub fn is_empty(&self) -> bool {
        self.timeout.is_none()
            && self.budget.is_none()
            && self.rate_limit.is_none()
            && !self.cancellation
    }
}

/// JSON configuration for [`TimeoutGuardrail`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeoutGuardrailOptions {
    #[serde(default)]
    pub max_timeout_ms: Option<u64>,
    #[serde(default)]
    pub require_explicit_timeout: bool,
}

/// JSON configuration for [`BudgetGuardrail`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetGuardrailOptions {
    #[serde(default)]
    pub max_task_steps: Option<usize>,
    #[serde(default)]
    pub require_explicit_budget: bool,
    #[serde(default)]
    pub allow_exhausted_orchestrator: bool,
}

/// JSON configuration for [`RateLimitGuardrail`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitGuardrailOptions {
    #[serde(default)]
    pub max_tasks: Option<usize>,
    #[serde(default)]
    pub window_ms: Option<u64>,
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
    /// Guardrail that rejected the task, if any.
    pub guardrail_name: Option<String>,
    /// Guardrail lifecycle stage where rejection occurred, if any.
    pub guardrail_stage: Option<String>,
}

#[cfg(feature = "multi-agent")]
impl OrchestratorOptions {
    /// Build a default task retry policy from options-level defaults.
    pub fn default_task_retry_policy(
        &self,
    ) -> antikythera_core::application::agent::multi_agent::TaskRetryPolicy {
        antikythera_core::application::agent::multi_agent::TaskRetryPolicy {
            max_retries: 0,
            backoff_ms: 0,
            condition: self.default_retry_condition.into(),
        }
    }

    /// Apply defaults into a task when it has no explicit retry policy.
    pub fn apply_to_task(
        &self,
        task: &mut antikythera_core::application::agent::multi_agent::AgentTask,
    ) {
        if task.retry_policy.is_none() {
            task.retry_policy = Some(self.default_task_retry_policy());
        }
    }

    /// Apply SDK options to a core orchestrator builder.
    pub fn apply_to_orchestrator<P>(
        &self,
        orchestrator: antikythera_core::application::agent::multi_agent::orchestrator::MultiAgentOrchestrator<P>,
    ) -> antikythera_core::application::agent::multi_agent::orchestrator::MultiAgentOrchestrator<P>
    where
        P: antikythera_core::infrastructure::model::ModelProvider + 'static,
    {
        orchestrator
            .with_budget(self.into())
            .with_default_retry_condition(self.default_retry_condition.into())
            .with_guardrails(self.guardrails.to_guardrail_chain())
    }
}

// ============================================================================
// Conversions between SDK types and core types
// ============================================================================

#[cfg(feature = "multi-agent")]
mod core_conversions {
    use super::*;
    use antikythera_core::application::agent::multi_agent::{
        BudgetGuardrail, BudgetSnapshot, CancellationGuardrail, GuardrailChain, OrchestratorBudget,
        RateLimitGuardrail, RetryCondition, TaskResult, TimeoutGuardrail,
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

    impl GuardrailOptions {
        /// Convert SDK guardrail options into a core guardrail chain.
        pub fn to_guardrail_chain(&self) -> GuardrailChain {
            let mut chain = GuardrailChain::new();

            if let Some(timeout) = &self.timeout
                && let Some(max_timeout_ms) = timeout.max_timeout_ms.filter(|value| *value > 0)
            {
                let mut guardrail = TimeoutGuardrail::new(max_timeout_ms);
                if timeout.require_explicit_timeout {
                    guardrail = guardrail.require_timeout();
                }
                chain.push(std::sync::Arc::new(guardrail));
            }

            if let Some(budget) = &self.budget {
                let mut guardrail = BudgetGuardrail::new();
                if let Some(max_task_steps) = budget.max_task_steps.filter(|value| *value > 0) {
                    guardrail = guardrail.with_max_task_steps(max_task_steps);
                }
                if budget.require_explicit_budget {
                    guardrail = guardrail.require_explicit_budget();
                }
                if budget.allow_exhausted_orchestrator {
                    guardrail = guardrail.allow_exhausted_orchestrator();
                }
                if budget.max_task_steps.is_some()
                    || budget.require_explicit_budget
                    || budget.allow_exhausted_orchestrator
                {
                    chain.push(std::sync::Arc::new(guardrail));
                }
            }

            if let Some(rate_limit) = &self.rate_limit
                && let (Some(max_tasks), Some(window_ms)) = (
                    rate_limit.max_tasks.filter(|value| *value > 0),
                    rate_limit.window_ms.filter(|value| *value > 0),
                )
            {
                chain.push(std::sync::Arc::new(RateLimitGuardrail::new(
                    max_tasks, window_ms,
                )));
            }

            if self.cancellation {
                chain.push(std::sync::Arc::new(CancellationGuardrail::new()));
            }

            chain
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
                step_budget_exhausted: snap
                    .max_total_steps
                    .is_some_and(|max| snap.consumed_steps >= max),
                task_budget_exhausted: snap
                    .max_total_tasks
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
                error_kind: result.error_kind.as_ref().map(|k| {
                    serde_json::to_value(k)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| format!("{:?}", k))
                }),
                is_transient: result.is_transient(),
                router_name: routing.map(|r| r.router_name.clone()),
                selected_agent_id: routing.map(|r| r.selected_agent_id.clone()),
                candidates_considered: routing.map(|r| r.candidates_considered),
                routing_reason: routing.and_then(|r| r.reason.clone()),
                concurrency_wait_ms: result.metadata.concurrency_wait_ms,
                budget_exhausted: result.metadata.budget_exhausted,
                guardrail_name: result.metadata.guardrail_name.clone(),
                guardrail_stage: result.metadata.guardrail_stage.clone(),
            }
        }
    }
}

// ============================================================================
// Host runtime hardening controls (native Rust surface)
// ============================================================================

#[cfg(feature = "multi-agent")]
#[derive(Debug, Clone, Default)]
struct HardeningRuntimeState {
    options: OrchestratorOptions,
    cancelled: bool,
    last_budget_snapshot: Option<antikythera_core::application::agent::multi_agent::BudgetSnapshot>,
}

#[cfg(feature = "multi-agent")]
static HARDENING_RUNTIME: LazyLock<Mutex<HardeningRuntimeState>> =
    LazyLock::new(|| Mutex::new(HardeningRuntimeState::default()));

#[cfg(feature = "multi-agent")]
fn with_hardening_runtime<T>(
    f: impl FnOnce(&mut HardeningRuntimeState) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = HARDENING_RUNTIME
        .lock()
        .map_err(|_| "hardening runtime lock poisoned".to_string())?;
    f(&mut guard)
}

/// Configure runtime hardening options from JSON.
///
/// This updates the host-visible defaults and clears cancellation state so the
/// next orchestrator run starts from a clean control plane.
#[cfg(feature = "multi-agent")]
pub fn configure_hardening(options_json: &str) -> Result<bool, String> {
    let options: OrchestratorOptions = serde_json::from_str(options_json)
        .map_err(|e| format!("Invalid OrchestratorOptions JSON: {e}"))?;

    if options.max_concurrent_tasks == Some(0) {
        return Err("max_concurrent_tasks must be > 0 if set".to_string());
    }
    if options.max_total_steps == Some(0) {
        return Err("max_total_steps must be > 0 if set".to_string());
    }
    if options.max_total_tasks == Some(0) {
        return Err("max_total_tasks must be > 0 if set".to_string());
    }
    validate_guardrail_options(&options.guardrails)?;

    with_hardening_runtime(|state| {
        state.options = options;
        state.cancelled = false;
        Ok(true)
    })
}

/// Mark orchestrator runtime as cancelled.
#[cfg(feature = "multi-agent")]
pub fn cancel_orchestrator() -> Result<bool, String> {
    with_hardening_runtime(|state| {
        state.cancelled = true;
        Ok(true)
    })
}

/// Update latest budget snapshot seen by host runtime.
#[cfg(feature = "multi-agent")]
pub fn update_monitor_budget_snapshot(
    snapshot: &antikythera_core::application::agent::multi_agent::BudgetSnapshot,
) -> Result<bool, String> {
    with_hardening_runtime(|state| {
        state.last_budget_snapshot = Some(snapshot.clone());
        Ok(true)
    })
}

/// Read monitor snapshot from the current host runtime state.
#[cfg(feature = "multi-agent")]
pub fn get_monitor_snapshot() -> Result<String, String> {
    with_hardening_runtime(|state| {
        let monitor = if let Some(snapshot) = state.last_budget_snapshot.as_ref() {
            OrchestratorMonitorSnapshot::from(snapshot).with_cancelled(state.cancelled)
        } else {
            OrchestratorMonitorSnapshot {
                max_total_steps: state.options.max_total_steps,
                max_total_tasks: state.options.max_total_tasks,
                max_concurrent_tasks: state.options.max_concurrent_tasks,
                cancelled: state.cancelled,
                ..OrchestratorMonitorSnapshot::default()
            }
        };
        serde_json::to_string(&monitor)
            .map_err(|e| format!("Failed to serialize monitor snapshot: {e}"))
    })
}

/// Decode a serialized `TaskResult` JSON into task detail JSON.
#[cfg(feature = "multi-agent")]
pub fn task_result_detail(task_result_json: &str) -> Result<String, String> {
    use antikythera_core::application::agent::multi_agent::TaskResult;

    let result: TaskResult = serde_json::from_str(task_result_json)
        .map_err(|e| format!("Invalid TaskResult JSON: {e}"))?;
    let detail = TaskResultDetail::from(&result);
    serde_json::to_string(&detail).map_err(|e| format!("Failed to serialize TaskResultDetail: {e}"))
}

/// Reset host runtime hardening state to defaults.
#[cfg(feature = "multi-agent")]
pub fn reset_hardening_runtime() -> Result<bool, String> {
    with_hardening_runtime(|state| {
        *state = HardeningRuntimeState::default();
        Ok(true)
    })
}

// ============================================================================
// FFI — Orchestrator options, monitoring, and task introspection
// ============================================================================

/// Return the default [`OrchestratorOptions`] as a JSON string.
///
/// Use this to obtain the canonical default configuration, then modify fields
/// as needed before passing to `mcp_build_orchestrator_budget`.
pub fn mcp_default_orchestrator_options() -> *mut c_char {
    serialize_result(&OrchestratorOptions::default())
}

/// Return default [`StreamingOptions`] as a JSON string.
pub fn mcp_default_streaming_options() -> *mut c_char {
    serialize_result(&StreamingOptions::default())
}

/// Validate a [`StreamingOptions`] JSON string.
///
/// Returns `{"valid": true}` or `{"valid": false, "error": "..."}`.
pub fn mcp_validate_streaming_options(options_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(options_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    };
    match serde_json::from_str::<StreamingOptions>(&json_str) {
        Ok(opts) => {
            let errors = validate_streaming_options_collect(&opts);
            if errors.is_empty() {
                to_c_string(r#"{"valid":true}"#)
            } else {
                serialize_result(&serde_json::json!({"valid": false, "errors": errors}))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    }
}

/// Validate an [`OrchestratorOptions`] JSON string.
///
/// Returns `{"valid": true}` or `{"valid": false, "error": "..."}`.
pub fn mcp_validate_orchestrator_options(options_json: *const c_char) -> *mut c_char {
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
            errors.extend(validate_guardrail_options_collect(&opts.guardrails));
            if errors.is_empty() {
                to_c_string(r#"{"valid":true}"#)
            } else {
                serialize_result(&serde_json::json!({"valid": false, "errors": errors}))
            }
        }
        Err(e) => to_c_string(&format!(r#"{{"valid":false,"error":"{}"}}"#, e)),
    }
}

/// Configure host runtime hardening options from JSON.
#[cfg(feature = "multi-agent")]
pub fn mcp_configure_hardening(options_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(options_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"success":false,"error":"{}"}}"#, e)),
    };

    match configure_hardening(&json_str) {
        Ok(success) => to_c_string(&format!(r#"{{"success":{success}}}"#)),
        Err(e) => to_c_string(&format!(r#"{{"success":false,"error":"{}"}}"#, e)),
    }
}

/// Mark active runtime as cancelled.
#[cfg(feature = "multi-agent")]
pub fn mcp_cancel_orchestrator() -> *mut c_char {
    match cancel_orchestrator() {
        Ok(success) => to_c_string(&format!(r#"{{"success":{success}}}"#)),
        Err(e) => to_c_string(&format!(r#"{{"success":false,"error":"{}"}}"#, e)),
    }
}

/// Return current monitor snapshot JSON from host runtime state.
#[cfg(feature = "multi-agent")]
pub fn mcp_get_monitor_snapshot() -> *mut c_char {
    match get_monitor_snapshot() {
        Ok(json) => to_c_string(&json),
        Err(e) => to_c_string(&format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Decode a serialized [`TaskResult`] JSON into a [`TaskResultDetail`] JSON
/// for easy routing/error introspection without requiring a live orchestrator.
///
/// Returns `{"error": "..."}` if the input cannot be parsed.
#[cfg(feature = "multi-agent")]
pub fn mcp_task_result_detail(task_result_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(task_result_json) {
        Ok(s) => s,
        Err(e) => return to_c_string(&format!(r#"{{"error":"{}"}}"#, e)),
    };
    match task_result_detail(&json_str) {
        Ok(json) => to_c_string(&json),
        Err(e) => to_c_string(&format!(r#"{{"error":"{}"}}"#, e)),
    }
}

/// Build an [`OrchestratorMonitorSnapshot`] from a [`BudgetSnapshot`] JSON
/// (obtained via `MultiAgentOrchestrator::budget_snapshot()`) and an optional
/// `cancelled` boolean.
///
/// This is a pure decode helper — it performs no I/O.
#[cfg(feature = "multi-agent")]
pub fn mcp_orchestrator_snapshot(
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
        Err(e) => to_c_string(&format!(
            r#"{{"error":"Invalid BudgetSnapshot JSON: {}"}}"#,
            e
        )),
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

#[cfg(feature = "multi-agent")]
fn validate_guardrail_options(guardrails: &GuardrailOptions) -> Result<(), String> {
    let errors = validate_guardrail_options_collect(guardrails);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn validate_guardrail_options_collect(guardrails: &GuardrailOptions) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(timeout) = &guardrails.timeout
        && timeout.max_timeout_ms == Some(0)
    {
        errors.push("guardrails.timeout.max_timeout_ms must be > 0 if set".to_string());
    }

    if let Some(budget) = &guardrails.budget
        && budget.max_task_steps == Some(0)
    {
        errors.push("guardrails.budget.max_task_steps must be > 0 if set".to_string());
    }

    if let Some(rate_limit) = &guardrails.rate_limit {
        if rate_limit.max_tasks == Some(0) {
            errors.push("guardrails.rate_limit.max_tasks must be > 0 if set".to_string());
        }
        if rate_limit.window_ms == Some(0) {
            errors.push("guardrails.rate_limit.window_ms must be > 0 if set".to_string());
        }
        if rate_limit.max_tasks.is_some() ^ rate_limit.window_ms.is_some() {
            errors.push("guardrails.rate_limit requires both max_tasks and window_ms".to_string());
        }
    }

    errors
}

fn validate_streaming_options_collect(options: &StreamingOptions) -> Vec<String> {
    let mut errors = Vec::new();

    if options.max_buffered_events == Some(0) {
        errors.push("max_buffered_events must be > 0 if set".to_string());
    }

    errors
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
        if !self
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(
                "Agent ID can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            );
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
        if let Some(temp) = self.temperature
            && !(0.0..=2.0).contains(&temp)
        {
            errors.push("Temperature must be between 0.0 and 2.0".to_string());
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
static AGENTS: LazyLock<Mutex<HashMap<String, AgentConfig>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Agent status tracking
static AGENT_STATUS: LazyLock<Mutex<HashMap<String, AgentStatus>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
pub fn mcp_register_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            return serialize_result(&AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            });
        }
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            return serialize_result(&AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            });
        }
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
pub fn mcp_unregister_agent(id: *const c_char) -> *mut c_char {
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
pub fn mcp_list_agents() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Get configuration for a specific agent
pub fn mcp_get_agent(id: *const c_char) -> *mut c_char {
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
pub fn mcp_get_agent_status() -> *mut c_char {
    match AGENT_STATUS.lock() {
        Ok(statuses) => {
            let status_list: Vec<&AgentStatus> = statuses.values().collect();
            serialize_result(&status_list)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Validate agent configuration without registering
pub fn mcp_validate_agent(config_json: *const c_char) -> *mut c_char {
    let json_str = match from_c_string(config_json) {
        Ok(s) => s,
        Err(e) => {
            return serialize_result(&AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            });
        }
    };

    let config: AgentConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => {
            return serialize_result(&AgentValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                agent_id: String::new(),
            });
        }
    };

    serialize_result(&config.validate())
}

/// Export all agents configuration as JSON
pub fn mcp_export_agents_config() -> *mut c_char {
    match AGENTS.lock() {
        Ok(agents) => {
            let configs: Vec<&AgentConfig> = agents.values().collect();
            serialize_result(&configs)
        }
        Err(e) => to_c_string(&format!(r#"{{"error": "{}"}}"#, e)),
    }
}

/// Import agents configuration from JSON
pub fn mcp_import_agents_config(config_json: *const c_char) -> *mut c_char {
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
