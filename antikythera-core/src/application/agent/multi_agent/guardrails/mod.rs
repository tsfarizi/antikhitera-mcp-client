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

pub mod builtin;
pub mod chain;

pub use builtin::{BudgetGuardrail, CancellationGuardrail, RateLimitGuardrail, TimeoutGuardrail};
pub use chain::{
    GuardrailChain, GuardrailContext, GuardrailRejection, GuardrailStage, TaskGuardrail,
};
