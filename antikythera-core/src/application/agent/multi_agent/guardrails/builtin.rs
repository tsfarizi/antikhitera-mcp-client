use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::super::registry::AgentProfile;
use super::super::task::{AgentTask, ErrorKind, TaskResult};
use super::chain::{GuardrailContext, GuardrailRejection, GuardrailStage, TaskGuardrail};

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

const fn default_true() -> bool {
    true
}

