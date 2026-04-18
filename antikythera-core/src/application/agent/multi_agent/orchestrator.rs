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
//! # async fn example(client: Arc<antikythera_core::application::client::McpClient<impl antikythera_core::infrastructure::model::ModelProvider + 'static>>) {
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

use tracing::{info, warn};

use super::execution::ExecutionMode;
use super::registry::{AgentProfile, AgentRegistry};
use super::router::{AgentRouter, FirstAvailableRouter};
use super::scheduler::TaskScheduler;
use super::task::{AgentTask, PipelineResult, TaskResult};
use crate::application::agent::{Agent, AgentOptions};
use crate::application::client::McpClient;
use crate::infrastructure::model::ModelProvider;

// ============================================================================
// Internal helpers
// ============================================================================

/// Execute a single task against a pre-resolved agent profile.
///
/// This free function is used both by the sequential `dispatch` path and
/// inside the closures passed to the scheduler.
async fn execute_task<P: ModelProvider>(
    client: Arc<McpClient<P>>,
    task: AgentTask,
    profile: AgentProfile,
) -> TaskResult {
    let agent = Agent::new(client);
    let max_steps = task.max_steps.or(profile.max_steps).unwrap_or(8);

    let options = AgentOptions {
        system_prompt: profile.system_prompt.clone(),
        session_id: task.session_id.clone(),
        max_steps,
        attachments: Vec::new(),
    };

    info!(
        task_id = %task.task_id,
        agent_id = %profile.id,
        "Dispatching task to agent"
    );

    match agent.run(task.input.clone(), options).await {
        Ok(outcome) => {
            info!(task_id = %task.task_id, agent_id = %profile.id, "Task completed");
            TaskResult::success(
                task.task_id,
                profile.id,
                outcome.response,
                outcome.steps.len(),
                outcome.session_id,
            )
        }
        Err(e) => {
            warn!(task_id = %task.task_id, agent_id = %profile.id, error = %e, "Task failed");
            TaskResult::failure(task.task_id, profile.id, e.to_string())
        }
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
/// # fn doc(client: Arc<antikythera_core::application::client::McpClient<impl antikythera_core::infrastructure::model::ModelProvider + 'static>>) {
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

    // ----------------------------------------------------------------
    // Dispatch
    // ----------------------------------------------------------------

    /// Dispatch a single task and wait for the result.
    ///
    /// The router is called to resolve the target agent.  If the router
    /// returns `None` a [`TaskResult::failure`] is returned immediately.
    pub async fn dispatch(&self, task: AgentTask) -> TaskResult {
        let profiles: Vec<&AgentProfile> = self.registry.list_profiles();
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
        execute_task(self.client.clone(), task, profile).await
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
        let prepared: Vec<(AgentTask, Option<AgentProfile>)> = tasks
            .into_iter()
            .map(|task| {
                let profile = self.router.route(&task, &profiles).cloned();
                (task, profile)
            })
            .collect();

        let client = self.client.clone();

        self.scheduler
            .run(prepared, move |(task, profile)| {
                let client = client.clone();
                async move {
                    match profile {
                        None => TaskResult::failure(
                            task.task_id.clone(),
                            task.agent_id.clone().unwrap_or_default(),
                            "No agent profile found for this task".to_string(),
                        ),
                        Some(p) => execute_task(client, task, p).await,
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
