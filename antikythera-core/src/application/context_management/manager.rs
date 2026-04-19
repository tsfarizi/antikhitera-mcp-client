/// Runtime context manager for session-level message filtering and truncation.
///
/// This module provides `RuntimeContextManager` which applies `ContextPolicy` to live
/// message histories, handling truncation, summarization, and policy mutations at runtime.
///
/// # Examples
///
/// ```
/// use antikythera_core::application::context_management::{RuntimeContextManager, ContextPolicy, TruncationStrategy};
/// use antikythera_core::domain::types::{ChatMessage, MessageRole};
///
/// let mut manager = RuntimeContextManager::new(ContextPolicy::new().with_max_history_messages(20));
///
/// // Apply current policy to messages
/// let messages = vec![
///     ChatMessage::new(MessageRole::System, "You are helpful"),
///     ChatMessage::new(MessageRole::User, "Hello"),
/// ];
/// let filtered = manager.apply_policy(&messages).expect("apply_policy failed");
/// ```
use super::policy::{ContextPolicy, SummarizationFn, TruncationStrategy};
use crate::domain::types::ChatMessage;
use std::sync::{Arc, Mutex};

/// Runtime context manager for session-level message management.
///
/// Handles application of `ContextPolicy` to message histories, including:
/// - Message filtering to respect token budgets
/// - Truncation strategy application
/// - System message preservation
/// - Runtime policy updates
pub struct RuntimeContextManager {
    policy: Arc<Mutex<ContextPolicy>>,
    summarization_callback: Arc<Mutex<Option<SummarizationFn>>>,
}

impl RuntimeContextManager {
    /// Create a new context manager with the given policy.
    pub fn new(policy: ContextPolicy) -> Self {
        Self {
            policy: Arc::new(Mutex::new(policy)),
            summarization_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Update the context policy at runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal mutex is poisoned.
    pub fn set_policy(&self, policy: ContextPolicy) -> Result<(), String> {
        *self.policy.lock().map_err(|e| format!("policy lock poisoned: {}", e))? = policy;
        Ok(())
    }

    /// Get the current policy (cloned).
    ///
    /// # Errors
    ///
    /// Returns an error if the internal mutex is poisoned.
    pub fn get_policy(&self) -> Result<ContextPolicy, String> {
        self.policy
            .lock()
            .map(|guard| guard.clone())
            .map_err(|e| format!("policy lock poisoned: {}", e))
    }

    /// Register a summarization callback for the Summarize truncation strategy.
    pub fn set_summarization_callback(&self, callback: SummarizationFn) {
        if let Ok(mut guard) = self.summarization_callback.lock() {
            *guard = Some(callback);
        }
    }

    /// Apply the current policy to a list of messages.
    ///
    /// This method:
    /// 1. Always retains system messages (up to their count)
    /// 2. Applies the configured truncation strategy
    /// 3. Respects token budget if configured
    /// 4. Returns a filtered vector of messages
    ///
    /// # Errors
    ///
    /// Returns an error if the policy lock is poisoned.
    pub fn apply_policy(&self, messages: &[ChatMessage]) -> Result<Vec<ChatMessage>, String> {
        let policy = self.policy.lock().map_err(|e| format!("policy lock poisoned: {}", e))?;

        // Separate system and non-system messages (cloned)
        let (mut system_msgs, mut non_system_msgs): (Vec<_>, Vec<_>) = messages
            .iter()
            .cloned()
            .partition(|m| m.role.as_str() == "system");

        // Always respect minimum system messages
        if system_msgs.len() > policy.min_system_messages {
            system_msgs.truncate(policy.min_system_messages);
        }

        // Truncate non-system messages if needed
        if non_system_msgs.len() > policy.max_history_messages {
            match policy.truncation_strategy {
                TruncationStrategy::KeepNewest => {
                    let skip_count = non_system_msgs.len().saturating_sub(policy.max_history_messages);
                    non_system_msgs = non_system_msgs
                        .into_iter()
                        .skip(skip_count)
                        .collect();
                }
                TruncationStrategy::KeepBalanced { head_ratio } => {
                    let head_count = ((policy.max_history_messages as f32 * head_ratio) as usize)
                        .min(non_system_msgs.len());
                    let tail_count = policy.max_history_messages.saturating_sub(head_count);
                    let tail_start = non_system_msgs.len().saturating_sub(tail_count);

                    let head: Vec<_> = non_system_msgs.iter().take(head_count).cloned().collect();
                    let tail: Vec<_> = non_system_msgs.iter().skip(tail_start).cloned().collect();

                    non_system_msgs = [head, tail].concat();
                }
                TruncationStrategy::Summarize => {
                    // Placeholder: would invoke summarization callback
                    // For now, fall back to KeepNewest
                    let skip_count = non_system_msgs.len().saturating_sub(policy.max_history_messages);
                    non_system_msgs = non_system_msgs
                        .into_iter()
                        .skip(skip_count)
                        .collect();
                }
            }
        }

        // Combine system and non-system messages
        let mut result = [system_msgs, non_system_msgs].concat();

        // Apply token budget if configured
        if let Some(budget) = policy.token_budget {
            while Self::estimate_tokens(&result) > budget && result.len() > policy.min_system_messages {
                // Remove oldest non-system message
                if let Some(pos) = result.iter().position(|m| m.role.as_str() != "system") {
                    result.remove(pos);
                } else {
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Estimate token count for a set of messages using simple heuristic.
    ///
    /// Uses: 1 token ≈ 4 characters (common approximation for English text).
    fn estimate_tokens(messages: &[ChatMessage]) -> usize {
        let total_chars: usize = messages.iter().map(|m| m.content().len()).sum();
        total_chars.div_ceil(4)
    }
}

impl Clone for RuntimeContextManager {
    fn clone(&self) -> Self {
        Self {
            policy: Arc::clone(&self.policy),
            summarization_callback: Arc::clone(&self.summarization_callback),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::MessageRole;

    fn make_message(role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage::new(role, content)
    }

    #[test]
    fn context_manager_preserves_system_messages() {
        let manager = RuntimeContextManager::new(ContextPolicy::default());
        let messages = vec![
            make_message(MessageRole::System, "You are helpful"),
            make_message(MessageRole::User, "Hello"),
            make_message(MessageRole::Assistant, "Hi there"),
        ];

        let result = manager.apply_policy(&messages).expect("apply_policy failed");

        // System message should be preserved
        assert!(result.iter().any(|m| m.role.as_str() == "system"));
    }

    #[test]
    fn context_manager_respects_max_history_messages() {
        let policy = ContextPolicy::new().with_max_history_messages(5);
        let manager = RuntimeContextManager::new(policy);

        let mut messages = (0..15).map(|i| make_message(MessageRole::User, &format!("msg {}", i))).collect::<Vec<_>>();
        messages.insert(0, make_message(MessageRole::System, "sys"));

        let result = manager.apply_policy(&messages).expect("apply_policy failed");

        // Should have system message + at most 5 user messages
        assert!(result.iter().filter(|m| m.role.as_str() == "user").count() <= 5);
    }

    #[test]
    fn context_manager_keep_newest_discards_oldest() {
        let policy = ContextPolicy::new()
            .with_max_history_messages(3)
            .with_truncation_strategy(TruncationStrategy::KeepNewest);
        let manager = RuntimeContextManager::new(policy);

        let messages = vec![
            make_message(MessageRole::User, "msg 0"),
            make_message(MessageRole::User, "msg 1"),
            make_message(MessageRole::User, "msg 2"),
            make_message(MessageRole::User, "msg 3"),
            make_message(MessageRole::User, "msg 4"),
        ];

        let result = manager.apply_policy(&messages).expect("apply_policy failed");

        // Should keep the last 3 messages
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].content(), "msg 2");
        assert_eq!(result[2].content(), "msg 4");
    }

    #[test]
    fn context_manager_keep_balanced_retains_head_and_tail() {
        let policy = ContextPolicy::new()
            .with_max_history_messages(6)
            .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.5 });
        let manager = RuntimeContextManager::new(policy);

        let messages = (0..20).map(|i| make_message(MessageRole::User, &format!("msg {}", i))).collect::<Vec<_>>();

        let result = manager.apply_policy(&messages).expect("apply_policy failed");

        // Should have ~3 from head and ~3 from tail
        assert_eq!(result.len(), 6);
        assert_eq!(result[0].content(), "msg 0"); // Head
        assert_eq!(result[5].content(), "msg 19"); // Tail
    }

    #[test]
    fn context_manager_respects_token_budget() {
        let policy = ContextPolicy::new()
            .with_max_history_messages(100)
            .with_token_budget(100); // Small budget
        let manager = RuntimeContextManager::new(policy);

        let messages = (0..50).map(|_| make_message(MessageRole::User, &"x".repeat(20))).collect::<Vec<_>>();

        let result = manager.apply_policy(&messages).expect("apply_policy failed");

        // Token count should be below budget
        let tokens = (result.iter().map(|m| m.content().len()).sum::<usize>() + 3) / 4;
        assert!(tokens <= 100);
    }

    #[test]
    fn context_manager_cloneable() {
        let manager1 = RuntimeContextManager::new(ContextPolicy::default());
        let manager2 = manager1.clone();

        let messages = vec![make_message(MessageRole::User, "test")];
        let result1 = manager1.apply_policy(&messages).expect("apply_policy failed");
        let result2 = manager2.apply_policy(&messages).expect("apply_policy failed");

        assert_eq!(result1.len(), result2.len());
    }
}
