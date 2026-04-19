//! Multi-agent orchestration.
//!
//! This module provides production-ready multi-agent scheduling, routing, and
//! pipeline execution on top of the existing single-agent [`Agent`] runner.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │          MultiAgentOrchestrator<P>            │
//! │  ┌─────────────┐  ┌──────────┐  ┌─────────┐  │
//! │  │AgentRegistry│  │TaskSched │  │Router   │  │
//! │  └─────────────┘  └──────────┘  └─────────┘  │
//! │         ↓               ↓            ↓        │
//! │    AgentProfile    ExecutionMode  AgentRouter  │
//! └──────────────────────────────────────────────┘
//!                   ↓
//!              Agent<P>::run(...)
//! ```
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
//! # async fn run(client: Arc<antikythera_core::application::client::McpClient<impl antikythera_core::infrastructure::model::ModelProvider + 'static>>) {
//! let orch = MultiAgentOrchestrator::new(client, ExecutionMode::Auto)
//!     .register_agent(AgentProfile {
//!         id: "reviewer".into(),
//!         name: "Reviewer".into(),
//!         role: "code-review".into(),
//!         system_prompt: Some("You are a code reviewer.".into()),
//!         max_steps: None,
//!     });
//!
//! let result = orch.dispatch(AgentTask::new("Review my PR")).await;
//! assert!(result.success);
//! # }
//! ```
//!
//! [`Agent`]: crate::application::agent::runner::Agent

pub mod budget;
pub mod cancellation;
pub mod execution;
pub mod guardrails;
pub mod orchestrator;
pub mod registry;
pub mod router;
pub mod scheduler;
pub mod task;

// ============================================================================
// Convenient re-exports (maintain backwards compatibility)
// ============================================================================

pub use budget::{BudgetSnapshot, OrchestratorBudget};
pub use cancellation::CancellationToken;
pub use execution::ExecutionMode;
pub use guardrails::{
    BudgetGuardrail, CancellationGuardrail, GuardrailChain, GuardrailContext, GuardrailRejection,
    GuardrailStage, RateLimitGuardrail, TaskGuardrail, TimeoutGuardrail,
};
pub use orchestrator::MultiAgentOrchestrator;
pub use registry::{
    AgentProfile, AgentRegistry, AgentRole, ContextId, MemoryConfig, MemoryProvider,
};
pub use router::{AgentRouter, DirectRouter, FirstAvailableRouter, RoleRouter, RoundRobinRouter};
pub use scheduler::TaskScheduler;
pub use task::{
    AgentTask, ErrorKind, PipelineResult, RetryCondition, RoutingDecision, TaskExecutionMetadata,
    TaskResult, TaskRetryPolicy,
};
