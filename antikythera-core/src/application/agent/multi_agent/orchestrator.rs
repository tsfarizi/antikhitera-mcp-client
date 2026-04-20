//! Multi-agent orchestrator.
//!
//! [`MultiAgentOrchestrator`] is the primary entry point for running multiple
//! agents over a shared [`McpClient`].  It combines an [`AgentRegistry`] (for
//! profile look-ups), a [`TaskScheduler`] (for execution-mode policy), and an
//! [`AgentRouter`] (for task → agent mapping) into a single, ergonomic API.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use antikythera_core::application::agent::multi_agent::{
//!     orchestrator::MultiAgentOrchestrator,
//!     registry::AgentProfile,
//!     task::AgentTask,
//!     execution::ExecutionMode,
//! };
//!
//! # async fn example(client: Arc<antikythera_core::application::client::McpClient<impl antikythera_core::application::model_provider::ModelProvider + 'static>>) {
//! let orchestrator = MultiAgentOrchestrator::new(client, ExecutionMode::Auto)
//!     .register_agent(AgentProfile {
//!         id: "reviewer".into(),
//!         name: "Code Reviewer".into(),
//!         role: "code-review".into(),
//!         system_prompt: Some("You are an expert code reviewer.".into()),
//!         max_steps: Some(10),
//!     });
//!
//! let task = AgentTask::new("Review this function for security issues");
//! let result = orchestrator.dispatch(task).await;
//! println!("Success: {}", result.success);
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{info, warn};

use super::budget::OrchestratorBudget;
use super::cancellation::CancellationToken;
use super::execution::ExecutionMode;
use super::guardrails::{GuardrailChain, GuardrailContext, GuardrailRejection};
use super::registry::{AgentProfile, AgentRegistry};
use super::router::{AgentRouter, FirstAvailableRouter};
use super::scheduler::TaskScheduler;
use super::task::{
    AgentTask, ErrorKind, PipelineResult, RetryCondition, RoutingDecision, TaskExecutionMetadata,
    TaskResult, TaskRetryPolicy,
};
use crate::application::agent::{Agent, AgentOptions};
use crate::application::client::McpClient;
use crate::application::model_provider::ModelProvider;

#[derive(Clone)]
struct ExecuteTaskRuntime {
    routing_decision: RoutingDecision,
    execution_mode: String,
    cancel_token: CancellationToken,
    budget: OrchestratorBudget,
    guardrails: GuardrailChain,
    concurrency_wait_ms: u64,
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Execute a single task against a pre-resolved agent profile.
///
/// This free function is used both by the sequential `dispatch` path and
/// inside the closures passed to the scheduler.  It is intentionally
/// free-standing so it can be cloned into async closures without carrying an
/// orchestrator reference.
///
/// # Hardening features wired here
/// - Deadline pre-check (fail fast before any work starts)
/// - Cancellation check (cancel token from orchestrator)
/// - `budget_steps` cap (min of task budget and profile max_steps)
/// - Per-task retry with `RetryCondition` gate
/// - Full `TaskExecutionMetadata` including `RoutingDecision`
/// - `ErrorKind` classification on every failure path
async fn execute_task<P: ModelProvider>(
    client: Arc<McpClient<P>>,
    task: AgentTask,
    profile: AgentProfile,
    runtime: ExecuteTaskRuntime,
) -> TaskResult {
    let started = Instant::now();
    let ExecuteTaskRuntime {
        routing_decision,
        execution_mode,
        cancel_token,
        budget,
        guardrails,
        concurrency_wait_ms,
    } = runtime;
    let max_steps = task.max_steps.or(profile.max_steps).unwrap_or(8);
    let budgeted_max_steps = task
        .budget_steps
        .map(|b| b.min(max_steps))
        .unwrap_or(max_steps);
    let retry_policy = task.retry_policy.clone().unwrap_or_default();

    let pre_context = GuardrailContext::new(
        budget.snapshot(),
        cancel_token.is_cancelled(),
        0,
        execution_mode.clone(),
    );

    if let Err(rejection) = guardrails.check_pre(&task, &profile, &pre_context) {
        return guardrail_failure_result(
            &task,
            &profile.id,
            &routing_decision,
            &execution_mode,
            concurrency_wait_ms,
            rejection,
        );
    }

    // ---- deadline pre-check ------------------------------------------------
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis() as i64;

    if let Some(deadline) = task.deadline_unix_ms
        && now_ms >= deadline
    {
        let metadata = TaskExecutionMetadata {
            deadline_exceeded: true,
            execution_mode: Some(execution_mode),
            correlation_id: task.correlation_id.clone(),
            routing_decision: Some(routing_decision),
            concurrency_wait_ms,
            error_kind: Some(ErrorKind::DeadlineExceeded),
            ..TaskExecutionMetadata::default()
        };
        return TaskResult::failure_with_kind(
            task.task_id,
            profile.id,
            "Task deadline exceeded before execution".to_string(),
            ErrorKind::DeadlineExceeded,
        )
        .with_metadata(metadata);
    }

    // ---- cancellation pre-check --------------------------------------------
    if cancel_token.is_cancelled() {
        let metadata = TaskExecutionMetadata {
            cancelled: true,
            execution_mode: Some(execution_mode),
            correlation_id: task.correlation_id.clone(),
            routing_decision: Some(routing_decision),
            concurrency_wait_ms,
            error_kind: Some(ErrorKind::Cancelled),
            ..TaskExecutionMetadata::default()
        };
        return TaskResult::failure_with_kind(
            task.task_id,
            profile.id,
            "Task cancelled before execution".to_string(),
            ErrorKind::Cancelled,
        )
        .with_metadata(metadata);
    }

    let mut attempt: u8 = 0;
    #[allow(unused_assignments)]
    let mut last_error: Option<(String, ErrorKind)> = None;

    loop {
        let mid_context = GuardrailContext::new(
            budget.snapshot(),
            cancel_token.is_cancelled(),
            attempt.saturating_add(1),
            execution_mode.clone(),
        );

        if let Err(rejection) = guardrails.check_mid(&task, &profile, &mid_context) {
            return guardrail_failure_result(
                &task,
                &profile.id,
                &routing_decision,
                &execution_mode,
                concurrency_wait_ms,
                rejection,
            );
        }

        // ---- per-attempt cancellation check --------------------------------
        if cancel_token.is_cancelled() {
            let metadata = TaskExecutionMetadata {
                attempt_count: attempt,
                duration_ms: started.elapsed().as_millis() as u64,
                cancelled: true,
                retry_applied: attempt > 1,
                execution_mode: Some(execution_mode),
                correlation_id: task.correlation_id,
                routing_decision: Some(routing_decision),
                concurrency_wait_ms,
                error_kind: Some(ErrorKind::Cancelled),
                ..TaskExecutionMetadata::default()
            };
            return TaskResult::failure_with_kind(
                task.task_id,
                profile.id,
                "Task cancelled during execution".to_string(),
                ErrorKind::Cancelled,
            )
            .with_metadata(metadata);
        }

        attempt = attempt.saturating_add(1);
        let agent = Agent::new(client.clone());
        let options = AgentOptions {
            system_prompt: profile.system_prompt.clone(),
            session_id: task.session_id.clone(),
            max_steps: budgeted_max_steps,
            attachments: Vec::new(),
        };

        info!(
            task_id = %task.task_id,
            agent_id = %profile.id,
            attempt = attempt,
            "Dispatching task to agent"
        );

        let run_future = agent.run(task.input.clone(), options);
        let run_result = if let Some(timeout_ms) = task.timeout_ms {
            match tokio::time::timeout(Duration::from_millis(timeout_ms), run_future).await {
                Ok(result) => result,
                Err(_) => {
                    // classify timeout as transient — could be a slow LLM
                    let should_retry = attempt <= retry_policy.max_retries
                        && retry_policy.condition != RetryCondition::Never;

                    if should_retry {
                        if retry_policy.backoff_ms > 0 {
                            sleep(Duration::from_millis(retry_policy.backoff_ms)).await;
                        }
                        continue;
                    }

                    let metadata = TaskExecutionMetadata {
                        attempt_count: attempt,
                        duration_ms: started.elapsed().as_millis() as u64,
                        timed_out: true,
                        retry_applied: attempt > 1,
                        execution_mode: Some(execution_mode),
                        correlation_id: task.correlation_id,
                        routing_decision: Some(routing_decision),
                        concurrency_wait_ms,
                        error_kind: Some(ErrorKind::Transient),
                        ..TaskExecutionMetadata::default()
                    };
                    return TaskResult::failure_with_kind(
                        task.task_id,
                        profile.id,
                        format!("Task timed out after {} ms", timeout_ms),
                        ErrorKind::Transient,
                    )
                    .with_metadata(metadata);
                }
            }
        } else {
            run_future.await
        };

        match run_result {
            Ok(outcome) => {
                info!(task_id = %task.task_id, agent_id = %profile.id, "Task completed");
                let metadata = TaskExecutionMetadata {
                    attempt_count: attempt,
                    duration_ms: started.elapsed().as_millis() as u64,
                    retry_applied: attempt > 1,
                    execution_mode: Some(execution_mode.clone()),
                    correlation_id: task.correlation_id.clone(),
                    routing_decision: Some(routing_decision.clone()),
                    concurrency_wait_ms,
                    ..TaskExecutionMetadata::default()
                };
                let result = TaskResult::success(
                    task.task_id.clone(),
                    profile.id.clone(),
                    outcome.response,
                    outcome.steps.len(),
                    outcome.session_id,
                )
                .with_metadata(metadata);

                let post_context = GuardrailContext::new(
                    budget.snapshot(),
                    cancel_token.is_cancelled(),
                    attempt,
                    execution_mode.clone(),
                );

                if let Err(rejection) =
                    guardrails.check_post(&task, &profile, &result, &post_context)
                {
                    return guardrail_failure_result(
                        &task,
                        &profile.id,
                        &routing_decision,
                        &execution_mode,
                        concurrency_wait_ms,
                        rejection,
                    );
                }

                return result;
            }
            Err(e) => {
                let err_str = e.to_string();
                // Heuristic: classify agent-side errors.
                // Callers can override by setting task.retry_policy.condition = OnTransient.
                let kind = classify_agent_error(&err_str);

                warn!(
                    task_id = %task.task_id,
                    agent_id = %profile.id,
                    error = %err_str,
                    attempt = attempt,
                    kind = ?kind,
                    "Task failed"
                );

                let should_retry = attempt <= retry_policy.max_retries
                    && match retry_policy.condition {
                        RetryCondition::Never => false,
                        RetryCondition::Always => true,
                        RetryCondition::OnTransient => kind == ErrorKind::Transient,
                    };

                last_error = Some((err_str, kind));

                if should_retry {
                    if retry_policy.backoff_ms > 0 {
                        sleep(Duration::from_millis(retry_policy.backoff_ms)).await;
                    }
                    continue;
                }
            }
        }

        break;
    }

    let (error_msg, error_kind) =
        last_error.unwrap_or_else(|| ("Task failed".to_string(), ErrorKind::Permanent));
    let metadata = TaskExecutionMetadata {
        attempt_count: attempt,
        duration_ms: started.elapsed().as_millis() as u64,
        retry_applied: attempt > 1,
        execution_mode: Some(execution_mode),
        correlation_id: task.correlation_id,
        routing_decision: Some(routing_decision),
        concurrency_wait_ms,
        error_kind: Some(error_kind.clone()),
        ..TaskExecutionMetadata::default()
    };

    TaskResult::failure_with_kind(task.task_id, profile.id, error_msg, error_kind)
        .with_metadata(metadata)
}

/// Classify an agent error string as transient or permanent.
///
/// This is a best-effort heuristic; callers can always override retry behaviour
/// via [`TaskRetryPolicy::condition`].
fn classify_agent_error(error: &str) -> ErrorKind {
    let lower = error.to_lowercase();
    if lower.contains("timeout")
        || lower.contains("rate limit")
        || lower.contains("503")
        || lower.contains("502")
        || lower.contains("429")
        || lower.contains("connection")
        || lower.contains("network")
        || lower.contains("temporarily")
    {
        ErrorKind::Transient
    } else {
        ErrorKind::Permanent
    }
}

// ============================================================================
// MultiAgentOrchestrator
// ============================================================================

/// Coordinates multiple agents across a shared [`McpClient`].
///
/// # Builder pattern
///
/// ```rust,no_run
/// # use std::sync::Arc;
/// # use antikythera_core::application::agent::multi_agent::{
/// #     orchestrator::MultiAgentOrchestrator,
/// #     registry::AgentProfile,
/// #     execution::ExecutionMode,
/// #     router::RoundRobinRouter,
/// # };
/// # fn doc(client: Arc<antikythera_core::application::client::McpClient<impl antikythera_core::application::model_provider::ModelProvider + 'static>>) {
/// let orchestrator = MultiAgentOrchestrator::new(client, ExecutionMode::Auto)
///     .register_agent(AgentProfile {
///         id: "a1".into(),
///         name: "Agent One".into(),
///         role: "general".into(),
///         system_prompt: None,
///         max_steps: None,
///     })
///     .register_agent(AgentProfile {
///         id: "a2".into(),
///         name: "Agent Two".into(),
///         role: "general".into(),
///         system_prompt: None,
///         max_steps: None,
///     })
///     .with_router(Arc::new(RoundRobinRouter::new()));
/// # }
/// ```
pub struct MultiAgentOrchestrator<P: ModelProvider> {
    registry: AgentRegistry<()>,
    scheduler: TaskScheduler,
    router: Arc<dyn AgentRouter>,
    client: Arc<McpClient<P>>,
    /// Orchestrator-level cancellation — shared with all running tasks.
    cancel_token: CancellationToken,
    /// Orchestrator-level concurrency and step budget guardrails.
    budget: OrchestratorBudget,
    /// Optional semaphore enforcing `budget.max_concurrent_tasks`.
    concurrency_sem: Option<Arc<Semaphore>>,
    /// Default retry condition for tasks without explicit retry policy.
    default_retry_condition: RetryCondition,
    /// Ordered guardrails evaluated around task execution.
    guardrails: GuardrailChain,
}

impl<P: ModelProvider + 'static> MultiAgentOrchestrator<P> {
    // ----------------------------------------------------------------
    // Constructors
    // ----------------------------------------------------------------

    /// Create a new orchestrator with an explicit execution mode.
    pub fn new(client: Arc<McpClient<P>>, mode: ExecutionMode) -> Self {
        Self {
            registry: AgentRegistry::new(),
            scheduler: TaskScheduler::new(mode),
            router: Arc::new(FirstAvailableRouter),
            client,
            cancel_token: CancellationToken::new(),
            budget: OrchestratorBudget::new(),
            concurrency_sem: None,
            default_retry_condition: RetryCondition::Always,
            guardrails: GuardrailChain::new(),
        }
    }

    /// Create an orchestrator with [`ExecutionMode::Auto`] (recommended default).
    pub fn with_auto_mode(client: Arc<McpClient<P>>) -> Self {
        Self::new(client, ExecutionMode::Auto)
    }

    // ----------------------------------------------------------------
    // Builder methods
    // ----------------------------------------------------------------

    /// Register an agent profile.
    ///
    /// Profiles with duplicate IDs silently replace the previous entry.
    pub fn register_agent(mut self, profile: AgentProfile) -> Self {
        self.registry.register(profile);
        self
    }

    /// Override the routing strategy.
    pub fn with_router(mut self, router: Arc<dyn AgentRouter>) -> Self {
        self.router = router;
        self
    }

    /// Override the execution mode after construction.
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.scheduler = TaskScheduler::new(mode);
        self
    }

    /// Set orchestrator-level budget guardrails.
    ///
    /// The budget is enforced in addition to per-task `budget_steps` and
    /// `ExecutionMode::Parallel { workers }`.  Setting
    /// `OrchestratorBudget::max_concurrent_tasks` installs a semaphore that
    /// limits concurrent executions across *all* dispatch paths.
    pub fn with_budget(mut self, budget: OrchestratorBudget) -> Self {
        self.concurrency_sem = budget
            .max_concurrent_tasks
            .map(|n| Arc::new(Semaphore::new(n.max(1))));
        self.budget = budget;
        self
    }

    /// Set orchestrator-level default retry condition.
    ///
    /// Applied only when a task does not define its own retry policy.
    pub fn with_default_retry_condition(mut self, condition: RetryCondition) -> Self {
        self.default_retry_condition = condition;
        self
    }

    /// Set the entire guardrail chain for this orchestrator.
    pub fn with_guardrails(mut self, guardrails: GuardrailChain) -> Self {
        self.guardrails = guardrails;
        self
    }

    /// Append a single guardrail to the existing chain.
    pub fn with_guardrail(mut self, guardrail: Arc<dyn super::guardrails::TaskGuardrail>) -> Self {
        self.guardrails.push(guardrail);
        self
    }

    // ----------------------------------------------------------------
    // Inspection
    // ----------------------------------------------------------------

    /// Return the number of registered agent profiles.
    pub fn agent_count(&self) -> usize {
        self.registry.count()
    }

    /// Return the current execution mode.
    pub fn execution_mode(&self) -> ExecutionMode {
        self.scheduler.mode
    }

    /// Return a snapshot of the current budget state.
    pub fn budget_snapshot(&self) -> super::budget::BudgetSnapshot {
        self.budget.snapshot()
    }

    /// Number of guardrails configured for this orchestrator.
    pub fn guardrail_count(&self) -> usize {
        self.guardrails.len()
    }

    // ----------------------------------------------------------------
    // Cancellation
    // ----------------------------------------------------------------

    /// Signal all running (and future) tasks to stop.
    ///
    /// After calling `cancel`, any task that checks the cancellation token
    /// will receive a [`TaskResult`] with `error_kind = Cancelled`.
    ///
    /// Cancellation is *cooperative* — tasks check the token between retry
    /// iterations, not mid-step.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Returns `true` if [`cancel`] has been called on this orchestrator.
    ///
    /// [`cancel`]: MultiAgentOrchestrator::cancel
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Return a child [`CancellationToken`] that can be stored or passed to
    /// other components.  Cancelling the orchestrator will propagate to all
    /// child tokens.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }

    // ----------------------------------------------------------------
    // Dispatch
    // ----------------------------------------------------------------

    /// Dispatch a single task and wait for the result.
    ///
    /// The router is called to resolve the target agent.  If the router
    /// returns `None` a [`TaskResult::failure`] is returned immediately.
    pub async fn dispatch(&self, task: AgentTask) -> TaskResult {
        let mut task = task;
        if task.retry_policy.is_none() {
            task.retry_policy = Some(TaskRetryPolicy {
                max_retries: 0,
                backoff_ms: 0,
                condition: self.default_retry_condition.clone(),
            });
        }

        // ---- budget guard -----------------------------------------------
        let dispatch_count = self.budget.record_task_dispatch();
        if self.budget.is_task_budget_exhausted() {
            let meta = TaskExecutionMetadata {
                budget_exhausted: true,
                execution_mode: Some(self.execution_mode().to_string()),
                correlation_id: task.correlation_id.clone(),
                error_kind: Some(ErrorKind::BudgetExhausted),
                ..TaskExecutionMetadata::default()
            };
            return TaskResult::failure_with_kind(
                task.task_id.clone(),
                task.agent_id.clone().unwrap_or_default(),
                format!(
                    "Orchestrator task budget exhausted (dispatched {})",
                    dispatch_count
                ),
                ErrorKind::BudgetExhausted,
            )
            .with_metadata(meta);
        }

        if self.budget.is_step_budget_exhausted() {
            let meta = TaskExecutionMetadata {
                budget_exhausted: true,
                execution_mode: Some(self.execution_mode().to_string()),
                correlation_id: task.correlation_id.clone(),
                error_kind: Some(ErrorKind::BudgetExhausted),
                ..TaskExecutionMetadata::default()
            };
            return TaskResult::failure_with_kind(
                task.task_id.clone(),
                task.agent_id.clone().unwrap_or_default(),
                format!(
                    "Orchestrator step budget exhausted ({} / {} steps consumed)",
                    self.budget.consumed_steps(),
                    self.budget.max_total_steps.unwrap_or(0),
                ),
                ErrorKind::BudgetExhausted,
            )
            .with_metadata(meta);
        }

        // ---- routing -------------------------------------------------------
        let profiles: Vec<&AgentProfile> = self.registry.list_profiles();
        let candidates = profiles.len();
        let profile = match self.router.route(&task, &profiles) {
            Some(p) => p.clone(),
            None => {
                return TaskResult::failure(
                    task.task_id.clone(),
                    task.agent_id.clone().unwrap_or_default(),
                    "No agent available to handle the task".to_string(),
                );
            }
        };

        let routing_decision = RoutingDecision {
            router_name: self.router.name().to_string(),
            selected_agent_id: profile.id.clone(),
            candidates_considered: candidates,
            reason: self.router.routing_reason(&task, &profile),
        };

        // ---- concurrency slot (optional semaphore) -------------------------
        let concurrency_wait_start = Instant::now();
        let _permit = if let Some(sem) = &self.concurrency_sem {
            Some(
                sem.clone()
                    .acquire_owned()
                    .await
                    .expect("orchestrator semaphore closed"),
            )
        } else {
            None
        };
        let concurrency_wait_ms = concurrency_wait_start.elapsed().as_millis() as u64;

        let result = execute_task(
            self.client.clone(),
            task,
            profile,
            ExecuteTaskRuntime {
                routing_decision,
                execution_mode: self.execution_mode().to_string(),
                cancel_token: self.cancel_token.child_token(),
                budget: self.budget.clone(),
                guardrails: self.guardrails.clone(),
                concurrency_wait_ms,
            },
        )
        .await;

        // ---- record steps consumed -----------------------------------------
        self.budget.record_steps(result.steps_used);

        result
    }

    /// Dispatch multiple tasks and collect all results.
    ///
    /// Routing is resolved for every task up-front before any task starts
    /// executing.  The actual execution order and degree of parallelism is
    /// determined by the configured [`ExecutionMode`].
    ///
    /// Results are returned in an unspecified order for `Auto` and `Parallel`
    /// modes, and in submission order for `Sequential` and `Concurrent` modes.
    pub async fn dispatch_many(&self, tasks: Vec<AgentTask>) -> Vec<TaskResult> {
        if tasks.is_empty() {
            return Vec::new();
        }

        // Resolve routing for all tasks before entering the scheduler
        let profiles: Vec<&AgentProfile> = self.registry.list_profiles();
        let candidates = profiles.len();
        let execution_mode = self.execution_mode().to_string();

        let prepared: Vec<(AgentTask, Option<AgentProfile>, RoutingDecision)> = tasks
            .into_iter()
            .map(|task| {
                let profile = self.router.route(&task, &profiles).cloned();
                let routing_decision = match &profile {
                    Some(p) => RoutingDecision {
                        router_name: self.router.name().to_string(),
                        selected_agent_id: p.id.clone(),
                        candidates_considered: candidates,
                        reason: self.router.routing_reason(&task, p),
                    },
                    None => RoutingDecision {
                        router_name: self.router.name().to_string(),
                        selected_agent_id: String::new(),
                        candidates_considered: candidates,
                        reason: Some("No matching agent found".to_string()),
                    },
                };
                (task, profile, routing_decision)
            })
            .collect();

        let client = self.client.clone();
        let cancel_token = self.cancel_token.clone();
        let budget = self.budget.clone();
        let concurrency_sem = self.concurrency_sem.clone();
        let default_retry_condition = self.default_retry_condition.clone();
        let guardrails = self.guardrails.clone();

        self.scheduler
            .run(prepared, move |(task, profile, routing_decision)| {
                let client = client.clone();
                let execution_mode = execution_mode.clone();
                let cancel_token = cancel_token.clone();
                let budget = budget.clone();
                let concurrency_sem = concurrency_sem.clone();
                let default_retry_condition = default_retry_condition.clone();
                let guardrails = guardrails.clone();
                async move {
                    let mut task = task;
                    if task.retry_policy.is_none() {
                        task.retry_policy = Some(TaskRetryPolicy {
                            max_retries: 0,
                            backoff_ms: 0,
                            condition: default_retry_condition,
                        });
                    }

                    // Budget guard per-task inside batch
                    let dispatch_count = budget.record_task_dispatch();
                    if budget.is_task_budget_exhausted() {
                        let meta = TaskExecutionMetadata {
                            budget_exhausted: true,
                            execution_mode: Some(execution_mode),
                            routing_decision: Some(routing_decision),
                            error_kind: Some(ErrorKind::BudgetExhausted),
                            ..TaskExecutionMetadata::default()
                        };
                        return TaskResult::failure_with_kind(
                            task.task_id.clone(),
                            task.agent_id.clone().unwrap_or_default(),
                            format!(
                                "Orchestrator task budget exhausted (dispatched {})",
                                dispatch_count
                            ),
                            ErrorKind::BudgetExhausted,
                        )
                        .with_metadata(meta);
                    }

                    // Concurrency slot
                    let concurrency_wait_start = Instant::now();
                    let _permit = if let Some(sem) = &concurrency_sem {
                        Some(
                            sem.clone()
                                .acquire_owned()
                                .await
                                .expect("orchestrator semaphore closed"),
                        )
                    } else {
                        None
                    };
                    let concurrency_wait_ms = concurrency_wait_start.elapsed().as_millis() as u64;

                    match profile {
                        None => TaskResult::failure(
                            task.task_id.clone(),
                            task.agent_id.clone().unwrap_or_default(),
                            "No agent profile found for this task".to_string(),
                        )
                        .with_metadata(TaskExecutionMetadata {
                            execution_mode: Some(execution_mode),
                            correlation_id: task.correlation_id,
                            routing_decision: Some(routing_decision),
                            ..TaskExecutionMetadata::default()
                        }),
                        Some(p) => {
                            let result = execute_task(
                                client,
                                task,
                                p,
                                ExecuteTaskRuntime {
                                    routing_decision,
                                    execution_mode,
                                    cancel_token,
                                    budget: budget.clone(),
                                    guardrails,
                                    concurrency_wait_ms,
                                },
                            )
                            .await;
                            budget.record_steps(result.steps_used);
                            result
                        }
                    }
                }
            })
            .await
    }

    /// Execute tasks as a sequential pipeline.
    ///
    /// Each task's output is prepended to the next task's input as context,
    /// enabling "chain-of-thought" style multi-step reasoning across agents.
    ///
    /// The pipeline short-circuits on the first failure: remaining tasks are
    /// not executed and the partial results are returned.
    pub async fn pipeline(&self, tasks: Vec<AgentTask>) -> PipelineResult {
        if tasks.is_empty() {
            return PipelineResult::from_results(Vec::new());
        }

        let mut results = Vec::with_capacity(tasks.len());
        let mut previous_output: Option<String> = None;

        for mut task in tasks {
            // Inject the previous step's output as leading context
            if let Some(prev) = previous_output.take() {
                task.input = format!(
                    "Previous step output:\n{prev}\n\n---\nCurrent task:\n{}",
                    task.input
                );
            }

            let result = self.dispatch(task).await;
            let success = result.success;
            let output_str = result.output.to_string();

            results.push(result);

            if !success {
                // Short-circuit on failure
                break;
            }

            previous_output = Some(output_str);
        }

        PipelineResult::from_results(results)
    }
}

fn guardrail_failure_result(
    task: &AgentTask,
    agent_id: &str,
    routing_decision: &RoutingDecision,
    execution_mode: &str,
    concurrency_wait_ms: u64,
    rejection: GuardrailRejection,
) -> TaskResult {
    let metadata = TaskExecutionMetadata {
        attempt_count: 0,
        duration_ms: 0,
        cancelled: rejection.error_kind == ErrorKind::Cancelled,
        retry_applied: false,
        execution_mode: Some(execution_mode.to_string()),
        correlation_id: task.correlation_id.clone(),
        routing_decision: Some(routing_decision.clone()),
        concurrency_wait_ms,
        budget_exhausted: rejection.error_kind == ErrorKind::BudgetExhausted,
        error_kind: Some(rejection.error_kind.clone()),
        guardrail_name: Some(rejection.guardrail_name.clone()),
        guardrail_stage: Some(rejection.stage.as_str().to_string()),
        ..TaskExecutionMetadata::default()
    };

    TaskResult::failure_with_kind(
        task.task_id.clone(),
        agent_id.to_string(),
        rejection.message,
        rejection.error_kind,
    )
    .with_metadata(metadata)
}
