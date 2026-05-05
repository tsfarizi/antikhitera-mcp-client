/// Context management policy for message truncation and summarization.
///
/// This module defines the `ContextPolicy` struct which controls how long conversation
/// histories are managed to stay within token budgets while preserving conversation quality.
///
/// # Examples
///
/// ```
/// use antikythera_core::application::context_management::{ContextPolicy, TruncationStrategy};
///
/// // Keep newest messages up to token budget
/// let policy = ContextPolicy {
///     max_history_messages: 50,
///     truncation_strategy: TruncationStrategy::KeepNewest,
///     min_system_messages: 2,
///     token_budget: Some(4000),
/// };
///
/// // Balanced strategy keeping both head and tail of conversation
/// let balanced_policy = ContextPolicy {
///     max_history_messages: 50,
///     truncation_strategy: TruncationStrategy::KeepBalanced { head_ratio: 0.3 },
///     min_system_messages: 2,
///     token_budget: Some(4000),
/// };
/// ```
use serde::{Deserialize, Serialize};

/// Truncation strategy for message history.
///
/// Defines how messages are removed when context window exceeds budget.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TruncationStrategy {
    /// Keep only the newest messages, discarding oldest first.
    #[default]
    KeepNewest,

    /// Keep both head (oldest) and tail (newest) of conversation, removing middle messages first.
    /// Useful for preserving conversation flow context.
    ///
    /// # Fields
    ///
    /// * `head_ratio` - Fraction of retained messages to keep from conversation head (0.0 to 1.0)
    KeepBalanced { head_ratio: f32 },

    /// Summarize older messages to make room for newer ones.
    /// Requires a summarization strategy callback to be registered.
    Summarize,
}

/// Summarization strategy callback type.
///
/// Takes a slice of messages and returns a summarized representation.
/// Implementation is host-provided via `RuntimeContextManager::set_summarization_callback`.
pub type SummarizationFn = fn(&[crate::domain::types::ChatMessage]) -> Option<String>;

/// Context management policy.
///
/// Configures how message history is managed during long agent conversations.
/// All fields have sensible defaults for typical use cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicy {
    /// Maximum number of messages to retain in history (excluding system messages).
    /// Default: 50
    pub max_history_messages: usize,

    /// Strategy for truncating messages when budget exceeded.
    /// Default: KeepNewest
    pub truncation_strategy: TruncationStrategy,

    /// Minimum number of system messages to always retain.
    /// Default: 1
    pub min_system_messages: usize,

    /// Token budget for entire message history (if Some, overrides max_history_messages).
    /// Estimated via simple char-count approximation (1 token ≈ 4 characters).
    /// Default: None (use max_history_messages only)
    pub token_budget: Option<usize>,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            max_history_messages: 50,
            truncation_strategy: TruncationStrategy::KeepNewest,
            min_system_messages: 1,
            token_budget: None,
        }
    }
}

impl ContextPolicy {
    /// Create a new context policy with all defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum history messages (fluent builder).
    pub fn with_max_history_messages(mut self, max: usize) -> Self {
        self.max_history_messages = max;
        self
    }

    /// Set truncation strategy (fluent builder).
    pub fn with_truncation_strategy(mut self, strategy: TruncationStrategy) -> Self {
        self.truncation_strategy = strategy;
        self
    }

    /// Set token budget (fluent builder).
    pub fn with_token_budget(mut self, budget: usize) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// Set minimum system messages to retain (fluent builder).
    pub fn with_min_system_messages(mut self, min: usize) -> Self {
        self.min_system_messages = min;
        self
    }
}


