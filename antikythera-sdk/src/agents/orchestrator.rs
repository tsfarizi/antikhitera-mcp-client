//! Orchestrator options and monitoring

use serde::{Deserialize, Serialize};
#[cfg(feature = "multi-agent")]
use std::sync::{LazyLock, Mutex};

/// SDK-level orchestrator configuration options.
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
    pub fn is_empty(&self) -> bool {
        self.timeout.is_none()
            && self.budget.is_none()
            && self.rate_limit.is_none()
            && !self.cancellation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeoutGuardrailOptions {
    #[serde(default)]
    pub max_timeout_ms: Option<u64>,
    #[serde(default)]
    pub require_explicit_timeout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetGuardrailOptions {
    #[serde(default)]
    pub max_task_steps: Option<usize>,
    #[serde(default)]
    pub require_explicit_budget: bool,
    #[serde(default)]
    pub allow_exhausted_orchestrator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitGuardrailOptions {
    #[serde(default)]
    pub max_tasks: Option<usize>,
    #[serde(default)]
    pub window_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetryConditionOption {
    #[default]
    Always,
    OnTransient,
    Never,
}

#[cfg(feature = "multi-agent")]
impl OrchestratorOptions {
    pub fn default_task_retry_policy(
        &self,
    ) -> antikythera_core::application::agent::multi_agent::TaskRetryPolicy {
        antikythera_core::application::agent::multi_agent::TaskRetryPolicy {
            max_retries: 0,
            backoff_ms: 0,
            condition: self.default_retry_condition.into(),
        }
    }

    pub fn apply_to_task(
        &self,
        task: &mut antikythera_core::application::agent::multi_agent::AgentTask,
    ) {
        if task.retry_policy.is_none() {
            task.retry_policy = Some(self.default_task_retry_policy());
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestratorMonitorSnapshot {
    pub consumed_steps: usize,
    pub dispatched_tasks: usize,
    pub max_total_steps: Option<usize>,
    pub max_total_tasks: Option<usize>,
    pub max_concurrent_tasks: Option<usize>,
    pub step_budget_exhausted: bool,
    pub task_budget_exhausted: bool,
    pub cancelled: bool,
}

impl OrchestratorMonitorSnapshot {
    /// Merge live cancellation state into an existing snapshot.
    pub fn with_cancelled(mut self, cancelled: bool) -> Self {
        self.cancelled = cancelled;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskResultDetail {
    pub error_kind: Option<String>,
    pub is_transient: bool,
    pub router_name: Option<String>,
    pub selected_agent_id: Option<String>,
    pub candidates_considered: Option<usize>,
    pub routing_reason: Option<String>,
    pub concurrency_wait_ms: u64,
    pub budget_exhausted: bool,
    pub guardrail_name: Option<String>,
    pub guardrail_stage: Option<String>,
}

#[cfg(feature = "multi-agent")]
#[derive(Debug, Clone, Default)]
pub struct HardeningRuntimeState {
    pub options: OrchestratorOptions,
    pub cancelled: bool,
    pub last_budget_snapshot:
        Option<antikythera_core::application::agent::multi_agent::BudgetSnapshot>,
}

#[cfg(feature = "multi-agent")]
pub static HARDENING_RUNTIME: LazyLock<Mutex<HardeningRuntimeState>> =
    LazyLock::new(|| Mutex::new(HardeningRuntimeState::default()));

#[cfg(feature = "multi-agent")]
pub fn with_hardening_runtime<T>(
    f: impl FnOnce(&mut HardeningRuntimeState) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = HARDENING_RUNTIME
        .lock()
        .map_err(|_| "hardening runtime lock poisoned".to_string())?;
    f(&mut guard)
}
