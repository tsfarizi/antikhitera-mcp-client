/// Advanced context management for message history and token budgeting.
///
/// This module provides tools for managing long conversation histories while respecting
/// token budgets and conversation quality constraints.
///
/// # Key Types
///
/// - [`ContextPolicy`] - Configuration for context management (truncation strategy, token budget)
/// - [`RuntimeContextManager`] - Runtime manager for applying policies to message histories
/// - [`TruncationStrategy`] - Strategy for removing messages when budget exceeded
///
/// # Examples
///
/// ```
/// use antikythera_core::application::context_management::{ContextPolicy, RuntimeContextManager, TruncationStrategy};
///
/// // Create a policy that keeps balanced head/tail and respects token budget
/// let policy = ContextPolicy::new()
///     .with_max_history_messages(50)
///     .with_token_budget(4000)
///     .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.3 });
///
/// let manager = RuntimeContextManager::new(policy);
///
/// // Apply policy to messages (typically in your agent loop)
/// // let filtered_messages = manager.apply_policy(&all_messages)?;
/// ```
pub mod manager;
pub mod policy;

pub use manager::RuntimeContextManager;
pub use policy::{ContextPolicy, SummarizationFn, TruncationStrategy};
