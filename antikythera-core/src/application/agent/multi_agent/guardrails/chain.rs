use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::super::budget::BudgetSnapshot;
use super::super::registry::AgentProfile;
use super::super::task::{AgentTask, ErrorKind, TaskResult};

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

fn current_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis() as i64
}
