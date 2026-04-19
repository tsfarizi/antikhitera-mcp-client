//! Guardrail composition for multi-agent task execution.
//!
//! Guardrails provide policy checks around task execution without changing the
//! underlying agent runtime. They are intentionally lightweight and fully
//! opt-in: when no guardrails are registered, the orchestrator behaves exactly
//! as before.
//!
//! # Built-ins
//!
//! - [`TimeoutGuardrail`] validates per-task timeout policy.
//! - [`BudgetGuardrail`] enforces explicit step ceilings and budget exhaustion.
//! - [`RateLimitGuardrail`] throttles task starts within a rolling time window.
//! - [`CancellationGuardrail`] blocks work when the orchestrator is cancelled.
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//! use antikythera_core::application::agent::multi_agent::guardrails::{
//!     BudgetGuardrail, GuardrailChain, TimeoutGuardrail,
//! };
//!
//! let guardrails = GuardrailChain::new()
//!     .with_guardrail(Arc::new(TimeoutGuardrail::new(5_000).require_timeout()))
//!     .with_guardrail(Arc::new(BudgetGuardrail::new().with_max_task_steps(8)));
//!
//! assert_eq!(guardrails.len(), 2);
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::budget::BudgetSnapshot;
use super::registry::AgentProfile;
use super::task::{AgentTask, ErrorKind, TaskResult};

/// Execution phase where a guardrail is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailStage {
    PreCheck,
    MidCheck,
    PostCheck,
}

impl GuardrailStage {
    /// Stable string form for telemetry and metadata.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreCheck => "pre_check",
            Self::MidCheck => "mid_check",
            Self::PostCheck => "post_check",
        }
    }
}

/// Structured guardrail rejection returned by built-in or custom policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailRejection {
    pub guardrail_name: String,
    pub stage: GuardrailStage,
    pub error_kind: ErrorKind,
    pub message: String,
}

impl GuardrailRejection {
    /// Create a new rejection value.
    pub fn new(
        guardrail_name: impl Into<String>,
        stage: GuardrailStage,
        error_kind: ErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            guardrail_name: guardrail_name.into(),
            stage,
            error_kind,
            message: message.into(),
        }
    }
}

/// Point-in-time information available to guardrails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailContext {
    pub budget_snapshot: BudgetSnapshot,
    pub cancelled: bool,
    pub now_unix_ms: i64,
    pub attempt: u8,
    pub execution_mode: String,
}

impl GuardrailContext {
    /// Create a context from the current clock and runtime state.
    pub fn new(
        budget_snapshot: BudgetSnapshot,
        cancelled: bool,
        attempt: u8,
        execution_mode: impl Into<String>,
    ) -> Self {
        Self {
            budget_snapshot,
            cancelled,
            now_unix_ms: current_unix_ms(),
            attempt,
            execution_mode: execution_mode.into(),
        }
    }

    /// Return `true` if the orchestrator step budget is exhausted.
    pub fn step_budget_exhausted(&self) -> bool {
        self.budget_snapshot
            .max_total_steps
            .is_some_and(|max| self.budget_snapshot.consumed_steps >= max)
    }

    /// Return `true` if the orchestrator task budget is exhausted.
    pub fn task_budget_exhausted(&self) -> bool {
        self.budget_snapshot
            .max_total_tasks
            .is_some_and(|max| self.budget_snapshot.dispatched_tasks >= max)
    }
}

/// Policy hook around task execution.
pub trait TaskGuardrail: Send + Sync {
    /// Human-readable stable name of the guardrail.
    fn name(&self) -> &'static str;

    /// Run before any task work starts.
    fn pre_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        _context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        Ok(())
    }

    /// Run before each attempt in the retry loop.
    fn mid_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        _context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        Ok(())
    }

    /// Run after a task result is produced.
    fn post_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        _result: &TaskResult,
        _context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        Ok(())
    }
}

/// Ordered composition of multiple guardrails.
#[derive(Clone, Default)]
pub struct GuardrailChain {
    guardrails: Vec<Arc<dyn TaskGuardrail>>,
}

impl GuardrailChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a guardrail to the chain.
    pub fn push(&mut self, guardrail: Arc<dyn TaskGuardrail>) {
        self.guardrails.push(guardrail);
    }

    /// Builder-style append.
    pub fn with_guardrail(mut self, guardrail: Arc<dyn TaskGuardrail>) -> Self {
        self.push(guardrail);
        self
    }

    /// Number of registered guardrails.
    pub fn len(&self) -> usize {
        self.guardrails.len()
    }

    /// `true` if the chain contains no policies.
    pub fn is_empty(&self) -> bool {
        self.guardrails.is_empty()
    }

    /// Evaluate all pre-check hooks in registration order.
    pub fn check_pre(
        &self,
        task: &AgentTask,
        profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        for guardrail in &self.guardrails {
            guardrail.pre_check(task, profile, context)?;
        }
        Ok(())
    }

    /// Evaluate all mid-check hooks in registration order.
    pub fn check_mid(
        &self,
        task: &AgentTask,
        profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        for guardrail in &self.guardrails {
            guardrail.mid_check(task, profile, context)?;
        }
        Ok(())
    }

    /// Evaluate all post-check hooks in registration order.
    pub fn check_post(
        &self,
        task: &AgentTask,
        profile: &AgentProfile,
        result: &TaskResult,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        for guardrail in &self.guardrails {
            guardrail.post_check(task, profile, result, context)?;
        }
        Ok(())
    }
}

/// Enforce timeout policy on each task before execution starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutGuardrail {
    pub max_timeout_ms: u64,
    #[serde(default)]
    pub require_explicit_timeout: bool,
}

impl TimeoutGuardrail {
    /// Create a timeout guardrail with a maximum permitted timeout.
    pub fn new(max_timeout_ms: u64) -> Self {
        Self {
            max_timeout_ms,
            require_explicit_timeout: false,
        }
    }

    /// Reject tasks that do not define `timeout_ms`.
    pub fn require_timeout(mut self) -> Self {
        self.require_explicit_timeout = true;
        self
    }
}

impl TaskGuardrail for TimeoutGuardrail {
    fn name(&self) -> &'static str {
        "timeout"
    }

    fn pre_check(
        &self,
        task: &AgentTask,
        _profile: &AgentProfile,
        _context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if self.require_explicit_timeout && task.timeout_ms.is_none() {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                "Task must define timeout_ms to satisfy timeout guardrail",
            ));
        }

        if let Some(timeout_ms) = task.timeout_ms
            && timeout_ms > self.max_timeout_ms
        {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                format!(
                    "Task timeout {} ms exceeds allowed maximum {} ms",
                    timeout_ms, self.max_timeout_ms
                ),
            ));
        }

        Ok(())
    }
}

/// Enforce per-task and orchestrator budget policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BudgetGuardrail {
    #[serde(default)]
    pub max_task_steps: Option<usize>,
    #[serde(default)]
    pub require_explicit_budget: bool,
    #[serde(default = "default_true")]
    pub reject_when_orchestrator_exhausted: bool,
}

impl BudgetGuardrail {
    /// Create a budget guardrail with no extra restrictions.
    pub fn new() -> Self {
        Self {
            max_task_steps: None,
            require_explicit_budget: false,
            reject_when_orchestrator_exhausted: true,
        }
    }

    /// Cap the maximum step budget any task may request.
    pub fn with_max_task_steps(mut self, max_task_steps: usize) -> Self {
        self.max_task_steps = Some(max_task_steps);
        self
    }

    /// Require `budget_steps` on every task.
    pub fn require_explicit_budget(mut self) -> Self {
        self.require_explicit_budget = true;
        self
    }

    /// Disable rejection when the shared orchestrator budget is already exhausted.
    pub fn allow_exhausted_orchestrator(mut self) -> Self {
        self.reject_when_orchestrator_exhausted = false;
        self
    }

    fn requested_steps(&self, task: &AgentTask, profile: &AgentProfile) -> Option<usize> {
        task.budget_steps.or(task.max_steps).or(profile.max_steps)
    }
}

impl TaskGuardrail for BudgetGuardrail {
    fn name(&self) -> &'static str {
        "budget"
    }

    fn pre_check(
        &self,
        task: &AgentTask,
        profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if self.reject_when_orchestrator_exhausted
            && (context.step_budget_exhausted() || context.task_budget_exhausted())
        {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::BudgetExhausted,
                "Orchestrator budget exhausted before task start",
            ));
        }

        if self.require_explicit_budget && task.budget_steps.is_none() {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                "Task must define budget_steps to satisfy budget guardrail",
            ));
        }

        if let Some(limit) = self.max_task_steps
            && let Some(requested) = self.requested_steps(task, profile)
            && requested > limit
        {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::BudgetExhausted,
                format!(
                    "Task requests {} steps but budget guardrail allows at most {}",
                    requested, limit
                ),
            ));
        }

        Ok(())
    }

    fn post_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        result: &TaskResult,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if let Some(limit) = self.max_task_steps
            && result.success
            && result.steps_used > limit
        {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PostCheck,
                ErrorKind::BudgetExhausted,
                format!(
                    "Task consumed {} steps which exceeds the guardrail limit {}",
                    result.steps_used, limit
                ),
            ));
        }

        if self.reject_when_orchestrator_exhausted
            && let Some(max_steps) = context.budget_snapshot.max_total_steps
            && context
                .budget_snapshot
                .consumed_steps
                .saturating_add(result.steps_used)
                > max_steps
        {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PostCheck,
                ErrorKind::BudgetExhausted,
                "Task result would exceed the orchestrator step budget",
            ));
        }

        Ok(())
    }
}

/// Reject tasks when they exceed a rolling dispatch rate.
#[derive(Debug, Clone)]
pub struct RateLimitGuardrail {
    pub max_tasks: usize,
    pub window_ms: u64,
    seen_at_ms: Arc<Mutex<VecDeque<i64>>>,
}

impl RateLimitGuardrail {
    /// Create a rate-limit guardrail.
    pub fn new(max_tasks: usize, window_ms: u64) -> Self {
        Self {
            max_tasks,
            window_ms,
            seen_at_ms: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn trim_expired(entries: &mut VecDeque<i64>, now_ms: i64, window_ms: u64) {
        let cutoff = now_ms.saturating_sub(window_ms as i64);
        while entries.front().is_some_and(|front| *front <= cutoff) {
            entries.pop_front();
        }
    }
}

impl TaskGuardrail for RateLimitGuardrail {
    fn name(&self) -> &'static str {
        "rate_limit"
    }

    fn pre_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        let mut entries = self.seen_at_ms.lock().map_err(|_| {
            GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                "Rate limit guardrail state lock poisoned",
            )
        })?;

        Self::trim_expired(&mut entries, context.now_unix_ms, self.window_ms);

        if entries.len() >= self.max_tasks {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Transient,
                format!(
                    "Rate limit exceeded: at most {} tasks per {} ms",
                    self.max_tasks, self.window_ms
                ),
            ));
        }

        entries.push_back(context.now_unix_ms);
        Ok(())
    }
}

/// Block work once the orchestrator cancellation token has been triggered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancellationGuardrail;

impl CancellationGuardrail {
    /// Create the guardrail.
    pub fn new() -> Self {
        Self
    }
}

impl TaskGuardrail for CancellationGuardrail {
    fn name(&self) -> &'static str {
        "cancellation"
    }

    fn pre_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if context.cancelled {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Cancelled,
                "Task cancelled before execution",
            ));
        }
        Ok(())
    }

    fn mid_check(
        &self,
        _task: &AgentTask,
        _profile: &AgentProfile,
        context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if context.cancelled {
            return Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::MidCheck,
                ErrorKind::Cancelled,
                "Task cancelled during execution",
            ));
        }
        Ok(())
    }
}

fn current_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis() as i64
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> AgentProfile {
        AgentProfile {
            id: "agent-1".to_string(),
            name: "Agent One".to_string(),
            role: "general".to_string(),
            system_prompt: None,
            max_steps: Some(8),
        }
    }

    fn context() -> GuardrailContext {
        GuardrailContext::new(
            BudgetSnapshot {
                max_concurrent_tasks: Some(2),
                max_total_steps: Some(20),
                max_total_tasks: Some(5),
                consumed_steps: 4,
                dispatched_tasks: 2,
            },
            false,
            1,
            "auto",
        )
    }

    #[test]
    fn guardrail_stage_as_str_is_stable() {
        assert_eq!(GuardrailStage::PreCheck.as_str(), "pre_check");
        assert_eq!(GuardrailStage::MidCheck.as_str(), "mid_check");
        assert_eq!(GuardrailStage::PostCheck.as_str(), "post_check");
    }

    #[test]
    fn guardrail_rejection_constructor_sets_fields() {
        let rejection = GuardrailRejection::new(
            "budget",
            GuardrailStage::PreCheck,
            ErrorKind::BudgetExhausted,
            "too many steps",
        );
        assert_eq!(rejection.guardrail_name, "budget");
        assert_eq!(rejection.stage, GuardrailStage::PreCheck);
        assert_eq!(rejection.error_kind, ErrorKind::BudgetExhausted);
        assert_eq!(rejection.message, "too many steps");
    }

    #[test]
    fn guardrail_context_detects_step_budget_exhaustion() {
        let exhausted = GuardrailContext::new(
            BudgetSnapshot {
                max_concurrent_tasks: None,
                max_total_steps: Some(10),
                max_total_tasks: None,
                consumed_steps: 10,
                dispatched_tasks: 0,
            },
            false,
            1,
            "auto",
        );
        assert!(exhausted.step_budget_exhausted());
    }

    #[test]
    fn guardrail_context_detects_task_budget_exhaustion() {
        let exhausted = GuardrailContext::new(
            BudgetSnapshot {
                max_concurrent_tasks: None,
                max_total_steps: None,
                max_total_tasks: Some(2),
                consumed_steps: 0,
                dispatched_tasks: 2,
            },
            false,
            1,
            "auto",
        );
        assert!(exhausted.task_budget_exhausted());
    }

    #[test]
    fn empty_guardrail_chain_is_empty() {
        let chain = GuardrailChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn guardrail_chain_with_guardrail_increments_len() {
        let chain = GuardrailChain::new().with_guardrail(Arc::new(TimeoutGuardrail::new(5_000)));
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn timeout_guardrail_allows_lower_timeout() {
        let task = AgentTask::new("work").with_timeout_ms(1_000);
        let result = TimeoutGuardrail::new(5_000).pre_check(&task, &profile(), &context());
        assert!(result.is_ok());
    }

    #[test]
    fn timeout_guardrail_rejects_missing_timeout_when_required() {
        let task = AgentTask::new("work");
        let err = TimeoutGuardrail::new(5_000)
            .require_timeout()
            .pre_check(&task, &profile(), &context())
            .expect_err("missing timeout should be rejected");
        assert_eq!(err.error_kind, ErrorKind::Permanent);
    }

    #[test]
    fn timeout_guardrail_rejects_timeout_above_limit() {
        let task = AgentTask::new("work").with_timeout_ms(10_000);
        let err = TimeoutGuardrail::new(5_000)
            .pre_check(&task, &profile(), &context())
            .expect_err("timeout above limit should be rejected");
        assert_eq!(err.guardrail_name, "timeout");
    }

    #[test]
    fn budget_guardrail_allows_task_within_limit() {
        let task = AgentTask::new("work").with_budget_steps(6);
        let result =
            BudgetGuardrail::new()
                .with_max_task_steps(8)
                .pre_check(&task, &profile(), &context());
        assert!(result.is_ok());
    }

    #[test]
    fn budget_guardrail_requires_explicit_budget_when_configured() {
        let task = AgentTask::new("work");
        let err = BudgetGuardrail::new()
            .require_explicit_budget()
            .pre_check(&task, &profile(), &context())
            .expect_err("missing budget should be rejected");
        assert_eq!(err.error_kind, ErrorKind::Permanent);
    }

    #[test]
    fn budget_guardrail_rejects_task_over_limit() {
        let task = AgentTask::new("work").with_budget_steps(12);
        let err = BudgetGuardrail::new()
            .with_max_task_steps(8)
            .pre_check(&task, &profile(), &context())
            .expect_err("over-budget task should be rejected");
        assert_eq!(err.error_kind, ErrorKind::BudgetExhausted);
    }

    #[test]
    fn budget_guardrail_rejects_exhausted_orchestrator_budget() {
        let ctx = GuardrailContext::new(
            BudgetSnapshot {
                max_concurrent_tasks: None,
                max_total_steps: Some(4),
                max_total_tasks: None,
                consumed_steps: 4,
                dispatched_tasks: 0,
            },
            false,
            1,
            "auto",
        );
        let err = BudgetGuardrail::new()
            .pre_check(&AgentTask::new("work"), &profile(), &ctx)
            .expect_err("exhausted budget should reject");
        assert_eq!(err.error_kind, ErrorKind::BudgetExhausted);
    }

    #[test]
    fn budget_guardrail_can_ignore_exhausted_orchestrator_budget() {
        let ctx = GuardrailContext::new(
            BudgetSnapshot {
                max_concurrent_tasks: None,
                max_total_steps: Some(4),
                max_total_tasks: None,
                consumed_steps: 4,
                dispatched_tasks: 0,
            },
            false,
            1,
            "auto",
        );
        let result = BudgetGuardrail::new()
            .allow_exhausted_orchestrator()
            .pre_check(&AgentTask::new("work"), &profile(), &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn budget_guardrail_post_check_rejects_result_above_limit() {
        let result = TaskResult::success(
            "task-1".to_string(),
            "agent-1".to_string(),
            serde_json::json!("ok"),
            9,
            "session-1".to_string(),
        );
        let err = BudgetGuardrail::new()
            .with_max_task_steps(8)
            .post_check(&AgentTask::new("work"), &profile(), &result, &context())
            .expect_err("result above limit should reject");
        assert_eq!(err.stage, GuardrailStage::PostCheck);
    }

    #[test]
    fn rate_limit_guardrail_allows_first_n_requests() {
        let guardrail = RateLimitGuardrail::new(2, 10_000);
        let task = AgentTask::new("work");
        assert!(guardrail.pre_check(&task, &profile(), &context()).is_ok());
        assert!(guardrail.pre_check(&task, &profile(), &context()).is_ok());
    }

    #[test]
    fn rate_limit_guardrail_rejects_when_window_is_full() {
        let guardrail = RateLimitGuardrail::new(1, 10_000);
        let task = AgentTask::new("work");
        assert!(guardrail.pre_check(&task, &profile(), &context()).is_ok());
        let err = guardrail
            .pre_check(&task, &profile(), &context())
            .expect_err("second request should be rate limited");
        assert_eq!(err.error_kind, ErrorKind::Transient);
    }

    #[test]
    fn cancellation_guardrail_allows_active_context() {
        let result =
            CancellationGuardrail::new().pre_check(&AgentTask::new("work"), &profile(), &context());
        assert!(result.is_ok());
    }

    #[test]
    fn cancellation_guardrail_rejects_cancelled_pre_check() {
        let cancelled = GuardrailContext::new(context().budget_snapshot, true, 1, "auto");
        let err = CancellationGuardrail::new()
            .pre_check(&AgentTask::new("work"), &profile(), &cancelled)
            .expect_err("cancelled context should reject");
        assert_eq!(err.error_kind, ErrorKind::Cancelled);
    }

    #[test]
    fn cancellation_guardrail_rejects_cancelled_mid_check() {
        let cancelled = GuardrailContext::new(context().budget_snapshot, true, 2, "auto");
        let err = CancellationGuardrail::new()
            .mid_check(&AgentTask::new("work"), &profile(), &cancelled)
            .expect_err("cancelled context should reject");
        assert_eq!(err.stage, GuardrailStage::MidCheck);
    }

    #[test]
    fn guardrail_chain_stops_on_first_rejection() {
        let chain = GuardrailChain::new()
            .with_guardrail(Arc::new(TimeoutGuardrail::new(5_000).require_timeout()))
            .with_guardrail(Arc::new(BudgetGuardrail::new().with_max_task_steps(8)));
        let err = chain
            .check_pre(&AgentTask::new("work"), &profile(), &context())
            .expect_err("first guardrail should reject");
        assert_eq!(err.guardrail_name, "timeout");
    }

    #[test]
    fn guardrail_chain_runs_post_checks() {
        let chain = GuardrailChain::new()
            .with_guardrail(Arc::new(BudgetGuardrail::new().with_max_task_steps(4)));
        let result = TaskResult::success(
            "task-1".to_string(),
            "agent-1".to_string(),
            serde_json::json!("ok"),
            5,
            "session-1".to_string(),
        );
        let err = chain
            .check_post(&AgentTask::new("work"), &profile(), &result, &context())
            .expect_err("post-check should reject oversized result");
        assert_eq!(err.guardrail_name, "budget");
    }
}
