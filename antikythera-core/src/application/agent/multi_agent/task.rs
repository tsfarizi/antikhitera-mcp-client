//! Task types for the multi-agent orchestrator.
//!
//! [`AgentTask`] represents a unit of work that an agent should process.
//! [`TaskResult`] captures the outcome.  [`PipelineResult`] aggregates the
//! results of a sequential pipeline of tasks.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ============================================================================
// RetryCondition
// ============================================================================

/// Controls under what circumstances the retry policy should engage.
///
/// Attach this to a [`TaskRetryPolicy`] to gate retries on error severity.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetryCondition {
    /// Retry on *any* failure — the default behaviour.
    #[default]
    Always,
    /// Only retry when the error is classified as [`ErrorKind::Transient`].
    /// Permanent errors, cancellations, and budget exhaustion are not retried.
    OnTransient,
    /// Never retry, even if `max_retries > 0`.
    Never,
}

// ============================================================================
// ErrorKind
// ============================================================================

/// Classifies the root cause of a task failure.
///
/// The orchestrator sets this on [`TaskResult`] so callers and downstream
/// systems can decide whether to retry, escalate, or discard the failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Temporary condition — network blip, resource contention, LLM rate-limit.
    /// These errors are candidates for retry with backoff.
    Transient,
    /// Permanent condition — invalid input, auth failure, unsupported operation.
    /// Retrying will not help.
    Permanent,
    /// Task was cancelled via [`CancellationToken::cancel`].
    ///
    /// [`CancellationToken::cancel`]: super::cancellation::CancellationToken::cancel
    Cancelled,
    /// Wall-clock deadline elapsed before or during execution.
    DeadlineExceeded,
    /// The orchestrator-level step or task budget was exhausted.
    BudgetExhausted,
}

// ============================================================================
// RoutingDecision  (introspection)
// ============================================================================

/// A record of the routing decision made for a single task.
///
/// Embedded in [`TaskExecutionMetadata`] to give operators visibility into
/// *why* a task landed on a particular agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingDecision {
    /// Name of the router implementation that made the decision.
    pub router_name: String,
    /// ID of the agent selected by the router.
    pub selected_agent_id: String,
    /// Number of registered agent profiles that were considered.
    pub candidates_considered: usize,
    /// Human-readable reason for the selection (router-specific).
    #[serde(default)]
    pub reason: Option<String>,
}

// ============================================================================
// AgentTask
// ============================================================================

/// A single unit of work dispatched to one agent.
///
/// Tasks are the primary input to the [`MultiAgentOrchestrator`].  They carry
/// the user-facing prompt, optional routing hints, and scheduling overrides.
///
/// # Builder pattern
///
/// ```rust
/// use antikythera_core::application::agent::multi_agent::task::AgentTask;
///
/// let task = AgentTask::new("Review this pull request")
///     .for_agent("code-reviewer")
///     .with_max_steps(12);
/// ```
///
/// [`MultiAgentOrchestrator`]: super::orchestrator::MultiAgentOrchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique identifier for this task (auto-generated if not set).
    pub task_id: String,

    /// Target agent ID.  When `None` the router selects an agent.
    #[serde(default)]
    pub agent_id: Option<String>,

    /// The prompt or instruction for the agent to process.
    pub input: String,

    /// Session ID.  When `None` a new session is created.
    #[serde(default)]
    pub session_id: Option<String>,

    /// Override the agent profile's `max_steps` for this particular task.
    #[serde(default)]
    pub max_steps: Option<usize>,

    /// Hard timeout for this task in milliseconds.
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Optional absolute deadline in unix milliseconds.
    #[serde(default)]
    pub deadline_unix_ms: Option<i64>,

    /// Optional retry policy for transient failures.
    #[serde(default)]
    pub retry_policy: Option<TaskRetryPolicy>,

    /// Optional budget cap for steps (guardrail).
    #[serde(default)]
    pub budget_steps: Option<usize>,

    /// Correlation ID propagated through task execution metadata.
    #[serde(default)]
    pub correlation_id: Option<String>,

    /// Arbitrary key-value metadata attached to the task.
    ///
    /// Useful for passing routing hints, correlation IDs, or tracing
    /// information through the pipeline without modifying the agent prompt.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl AgentTask {
    /// Create a new task with an auto-generated ID.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            agent_id: None,
            input: input.into(),
            session_id: None,
            max_steps: None,
            timeout_ms: None,
            deadline_unix_ms: None,
            retry_policy: None,
            budget_steps: None,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Route this task to a specific agent by ID.
    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Pin this task to an existing session.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set a per-task step limit.
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Set a task timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set a retry policy.
    pub fn with_retry_policy(mut self, retry_policy: TaskRetryPolicy) -> Self {
        self.retry_policy = Some(retry_policy);
        self
    }

    /// Set a step budget guardrail.
    pub fn with_budget_steps(mut self, budget_steps: usize) -> Self {
        self.budget_steps = Some(budget_steps);
        self
    }

    /// Set task correlation ID.
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Attach arbitrary metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.metadata.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(Value::Null),
        );
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskRetryPolicy {
    /// Additional attempts after the first execution.
    #[serde(default)]
    pub max_retries: u8,
    /// Base backoff in milliseconds between retries.
    #[serde(default)]
    pub backoff_ms: u64,
    /// Gate controlling when retries are attempted.
    /// Defaults to [`RetryCondition::Always`].
    #[serde(default)]
    pub condition: RetryCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskExecutionMetadata {
    #[serde(default)]
    pub attempt_count: u8,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub timed_out: bool,
    #[serde(default)]
    pub deadline_exceeded: bool,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub retry_applied: bool,
    #[serde(default)]
    pub routed_by: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    // ---- error classification ------------------------------------------------
    /// Root-cause category of the failure, if the task did not succeed.
    #[serde(default)]
    pub error_kind: Option<ErrorKind>,
    // ---- routing introspection -----------------------------------------------
    /// Full routing decision record for this task.
    #[serde(default)]
    pub routing_decision: Option<RoutingDecision>,
    // ---- scheduling introspection --------------------------------------------
    /// Milliseconds spent waiting for a concurrency slot before execution began.
    #[serde(default)]
    pub concurrency_wait_ms: u64,
    /// Whether execution was blocked by an exhausted orchestrator budget.
    #[serde(default)]
    pub budget_exhausted: bool,
    /// Guardrail name that rejected or transformed the task, if any.
    #[serde(default)]
    pub guardrail_name: Option<String>,
    /// Guardrail stage (`pre_check`, `mid_check`, `post_check`) if triggered.
    #[serde(default)]
    pub guardrail_stage: Option<String>,
}

// ============================================================================
// TaskResult
// ============================================================================

/// The outcome of executing a single [`AgentTask`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// ID of the original [`AgentTask`].
    pub task_id: String,

    /// ID of the agent that processed the task.
    pub agent_id: String,

    /// Structured output from the agent (the LLM's parsed response).
    pub output: Value,

    /// `true` if the agent completed without error.
    pub success: bool,

    /// Human-readable error message when `success` is `false`.
    #[serde(default)]
    pub error: Option<String>,

    /// Structured error category (set on failure).
    #[serde(default)]
    pub error_kind: Option<ErrorKind>,

    /// Number of reasoning steps consumed.
    pub steps_used: usize,

    /// Session ID used for this task.
    pub session_id: String,

    /// Production introspection metadata for this task execution.
    #[serde(default)]
    pub metadata: TaskExecutionMetadata,
}

impl TaskResult {
    /// Construct a successful result.
    pub fn success(
        task_id: String,
        agent_id: String,
        output: Value,
        steps_used: usize,
        session_id: String,
    ) -> Self {
        Self {
            task_id,
            agent_id,
            output,
            success: true,
            error: None,
            error_kind: None,
            steps_used,
            session_id,
            metadata: TaskExecutionMetadata::default(),
        }
    }

    /// Construct a failure result with a classified error kind.
    pub fn failure_with_kind(
        task_id: String,
        agent_id: String,
        error: String,
        kind: ErrorKind,
    ) -> Self {
        Self {
            task_id,
            agent_id,
            output: Value::Null,
            success: false,
            error: Some(error),
            error_kind: Some(kind),
            steps_used: 0,
            session_id: String::new(),
            metadata: TaskExecutionMetadata::default(),
        }
    }

    /// Construct a failure result (error kind defaults to `Permanent`).
    pub fn failure(task_id: String, agent_id: String, error: String) -> Self {
        Self::failure_with_kind(task_id, agent_id, error, ErrorKind::Permanent)
    }

    pub fn with_metadata(mut self, metadata: TaskExecutionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Returns `true` if the error kind is transient (candidate for retry).
    pub fn is_transient(&self) -> bool {
        matches!(self.error_kind, Some(ErrorKind::Transient))
    }
}

// ============================================================================
// PipelineResult
// ============================================================================

/// The combined outcome of a sequential pipeline of tasks where each task's
/// output can feed into the next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Individual results in pipeline order.
    pub task_results: Vec<TaskResult>,

    /// Output of the final successful task, or `Value::Null` if empty.
    pub final_output: Value,

    /// Total reasoning steps consumed across all tasks.
    pub total_steps: usize,

    /// `true` if every task in the pipeline succeeded.
    pub success: bool,

    /// Error message of the first failing task (if any).
    #[serde(default)]
    pub error: Option<String>,
}

impl PipelineResult {
    /// Build a `PipelineResult` from an ordered list of [`TaskResult`]s.
    pub fn from_results(results: Vec<TaskResult>) -> Self {
        let total_steps = results.iter().map(|r| r.steps_used).sum();
        let success = results.iter().all(|r| r.success);
        let final_output = results
            .last()
            .map(|r| r.output.clone())
            .unwrap_or(Value::Null);
        let error = if !success {
            results
                .iter()
                .find(|r| !r.success)
                .and_then(|r| r.error.clone())
        } else {
            None
        };
        Self {
            task_results: results,
            final_output,
            total_steps,
            success,
            error,
        }
    }
}
