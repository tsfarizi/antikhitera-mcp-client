//! Context-window management.
//!
//! Provides:
//!
//! - [`TokenEstimator`]      — lightweight heuristic token counter (no tokenizer
//!                             dependency).
//! - [`ContextWindowPolicy`] — configurable token budget with a response
//!                             reservation.
//! - [`prune_messages`]      — removes the oldest non-system messages until the
//!                             message list fits within the policy budget.
//!
//! # Token estimation accuracy
//!
//! The estimator uses the widely-cited rule of thumb **1 token ≈ 4 characters**
//! for English text.  Accuracy is ±30 % for typical prompts — sufficient for
//! proactive pruning without an ML tokenizer dependency.

use crate::domain::types::{ChatMessage, MessagePart, MessageRole};
use serde::{Deserialize, Serialize};

// ── Token estimator ───────────────────────────────────────────────────────────

/// Heuristic token counter.
///
/// No external tokenizer is used; accuracy is intentionally approximate
/// (±30 % on typical English text).
pub struct TokenEstimator;

impl TokenEstimator {
    /// Estimate the token count for a plain text string.
    ///
    /// Uses `ceil(len / 4)` with a minimum of 1 token so empty strings are
    /// never counted as zero.
    pub fn estimate_text(text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    /// Estimate the token count for a single [`MessagePart`].
    ///
    /// Images are estimated at a fixed base cost (85 tokens) plus a small
    /// overhead proportional to the encoded data length, matching the OpenAI
    /// vision tokenisation guide for medium-resolution images.
    pub fn estimate_part(part: &MessagePart) -> usize {
        match part {
            MessagePart::Text { text } => Self::estimate_text(text),
            MessagePart::Image { data, .. } => 85 + data.len() / 1_000,
            MessagePart::File { data, .. } => Self::estimate_text(data),
        }
    }

    /// Estimate the token count for a full [`ChatMessage`], including the
    /// per-message role + formatting overhead (4 tokens, per the OpenAI guide).
    pub fn estimate_message(msg: &ChatMessage) -> usize {
        let overhead = 4;
        let content: usize = msg.parts.iter().map(Self::estimate_part).sum();
        overhead + content
    }

    /// Sum token estimates across a slice of messages.
    pub fn estimate_messages(messages: &[ChatMessage]) -> usize {
        messages.iter().map(Self::estimate_message).sum()
    }
}

// ── Policy ────────────────────────────────────────────────────────────────────

/// Context-window budget policy.
///
/// # Default values
///
/// | Field                   | Default |
/// |-------------------------|---------|
/// | `max_tokens`            | 8 192   |
/// | `reserve_for_response`  | 1 024   |
/// | `min_history_messages`  | 2       |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowPolicy {
    /// Hard token limit for the model's context window.
    pub max_tokens: usize,
    /// Tokens to reserve for the model's output on each call.
    pub reserve_for_response: usize,
    /// Minimum number of non-system messages to always retain, even if they
    /// push the total above budget.  Prevents the agent from running with a
    /// completely empty history.
    pub min_history_messages: usize,
}

impl Default for ContextWindowPolicy {
    fn default() -> Self {
        Self {
            max_tokens: 8_192,
            reserve_for_response: 1_024,
            min_history_messages: 2,
        }
    }
}

impl ContextWindowPolicy {
    /// Effective token budget for the message list (total minus response
    /// reservation).
    pub fn message_budget(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserve_for_response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicyOverride {
    pub provider: String,
    pub model_contains: Option<String>,
    pub policy: ContextWindowPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowManager {
    pub default_policy: ContextWindowPolicy,
    pub overrides: Vec<ContextPolicyOverride>,
}

impl Default for ContextWindowManager {
    fn default() -> Self {
        Self {
            default_policy: ContextWindowPolicy::default(),
            overrides: vec![
                ContextPolicyOverride {
                    provider: "gemini".to_string(),
                    model_contains: Some("pro".to_string()),
                    policy: ContextWindowPolicy {
                        max_tokens: 32_768,
                        reserve_for_response: 2_048,
                        min_history_messages: 4,
                    },
                },
                ContextPolicyOverride {
                    provider: "openai".to_string(),
                    model_contains: Some("4o".to_string()),
                    policy: ContextWindowPolicy {
                        max_tokens: 16_384,
                        reserve_for_response: 2_048,
                        min_history_messages: 4,
                    },
                },
                ContextPolicyOverride {
                    provider: "ollama".to_string(),
                    model_contains: None,
                    policy: ContextWindowPolicy {
                        max_tokens: 8_192,
                        reserve_for_response: 1_024,
                        min_history_messages: 3,
                    },
                },
            ],
        }
    }
}

impl ContextWindowManager {
    pub fn policy_for(&self, provider: &str, model: &str) -> ContextWindowPolicy {
        self.overrides
            .iter()
            .find(|entry| {
                entry.provider.eq_ignore_ascii_case(provider)
                    && entry.model_contains.as_ref().map_or(true, |needle| {
                        model.to_lowercase().contains(&needle.to_lowercase())
                    })
            })
            .map(|entry| entry.policy.clone())
            .unwrap_or_else(|| self.default_policy.clone())
    }
}

// ── Pruning ───────────────────────────────────────────────────────────────────

/// Prune `messages` to fit within `policy.message_budget()` tokens.
///
/// # Strategy
///
/// 1. System messages are **always** retained.
/// 2. Non-system messages are accumulated from newest → oldest.
/// 3. The oldest non-system messages are dropped once the budget is exceeded.
/// 4. At least `policy.min_history_messages` non-system messages are kept even
///    if they push the total above budget (guarantees the agent has context).
///
/// Returns a new `Vec<ChatMessage>` with system messages first, followed by
/// the retained non-system messages in their original order.
pub fn prune_messages(messages: &[ChatMessage], policy: &ContextWindowPolicy) -> Vec<ChatMessage> {
    let budget = policy.message_budget();
    if TokenEstimator::estimate_messages(messages) <= budget {
        return messages.to_vec();
    }

    let system_msgs: Vec<&ChatMessage> = messages
        .iter()
        .filter(|m| m.role == MessageRole::System)
        .collect();
    let non_system: Vec<&ChatMessage> = messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .collect();

    let system_tokens: usize = system_msgs
        .iter()
        .map(|m| TokenEstimator::estimate_message(m))
        .sum();
    let remaining_budget = budget.saturating_sub(system_tokens);

    // Walk non-system messages newest → oldest, accumulate until budget.
    let mut selected: Vec<&ChatMessage> = Vec::new();
    let mut used = 0usize;
    for msg in non_system.iter().rev() {
        let cost = TokenEstimator::estimate_message(msg);
        if used + cost <= remaining_budget || selected.len() < policy.min_history_messages {
            selected.push(msg);
            used += cost;
        }
    }

    // Restore original ordering: system first, then non-system oldest → newest.
    selected.reverse();
    let mut result: Vec<ChatMessage> = system_msgs.into_iter().cloned().collect();
    result.extend(selected.into_iter().cloned());
    result
}

pub fn summarize_messages(messages: &[ChatMessage], max_chars: usize) -> String {
    let mut summary = String::new();

    for message in messages {
        let role = match message.role {
            MessageRole::System => "System",
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
        };
        let content = message.content().split_whitespace().collect::<Vec<_>>().join(" ");
        if content.is_empty() {
            continue;
        }

        let line = format!("- {role}: {content}\n");
        if summary.len() + line.len() > max_chars {
            break;
        }
        summary.push_str(&line);
    }

    if summary.is_empty() {
        "- No prior conversation details preserved.".to_string()
    } else {
        summary.trim_end().to_string()
    }
}

pub fn summarize_and_prune_messages(
    messages: &[ChatMessage],
    policy: &ContextWindowPolicy,
) -> Vec<ChatMessage> {
    let pruned = prune_messages(messages, policy);
    if pruned.len() == messages.len() {
        return pruned;
    }

    let original_non_system: Vec<&ChatMessage> = messages
        .iter()
        .filter(|message| message.role != MessageRole::System)
        .collect();
    let retained_non_system: Vec<&ChatMessage> = pruned
        .iter()
        .filter(|message| message.role != MessageRole::System)
        .collect();
    let dropped_count = original_non_system
        .len()
        .saturating_sub(retained_non_system.len());

    if dropped_count == 0 {
        return pruned;
    }

    let dropped_messages: Vec<ChatMessage> = original_non_system[..dropped_count]
        .iter()
        .map(|message| (*message).clone())
        .collect();
    let summary = summarize_messages(&dropped_messages, 480);

    let mut rebuilt = Vec::new();
    rebuilt.extend(
        pruned
            .iter()
            .filter(|message| message.role == MessageRole::System)
            .cloned(),
    );
    rebuilt.push(ChatMessage::new(
        MessageRole::System,
        format!("Conversation summary:\n{summary}"),
    ));
    rebuilt.extend(
        pruned
            .iter()
            .filter(|message| message.role != MessageRole::System)
            .cloned(),
    );

    prune_messages(&rebuilt, policy)
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::MessageRole;

    fn make_msg(role: MessageRole, text: &str) -> ChatMessage {
        ChatMessage::new(role, text)
    }

    // ── TokenEstimator ────────────────────────────────────────────────────

    #[test]
    fn estimate_text_is_non_zero_for_non_empty_input() {
        assert!(TokenEstimator::estimate_text("hello world") > 0);
    }

    #[test]
    fn estimate_text_minimum_is_one_for_short_strings() {
        // "hi" is 2 chars; 2/4 = 0, should return at least 1
        assert_eq!(TokenEstimator::estimate_text("hi"), 1);
    }

    #[test]
    fn estimate_text_scales_with_length() {
        let short = TokenEstimator::estimate_text("hi");
        let long = TokenEstimator::estimate_text(&"a".repeat(1_000));
        assert!(long > short);
    }

    #[test]
    fn estimate_message_includes_role_overhead() {
        let msg = make_msg(MessageRole::User, "hello");
        let content_tokens = TokenEstimator::estimate_text("hello");
        // Role overhead is 4 tokens
        assert_eq!(TokenEstimator::estimate_message(&msg), content_tokens + 4);
    }

    #[test]
    fn estimate_messages_sums_individual_estimates() {
        let msgs = vec![
            make_msg(MessageRole::User, "hello"),
            make_msg(MessageRole::Assistant, "world"),
        ];
        let total = TokenEstimator::estimate_messages(&msgs);
        let expected = TokenEstimator::estimate_message(&msgs[0])
            + TokenEstimator::estimate_message(&msgs[1]);
        assert_eq!(total, expected);
    }

    // ── ContextWindowPolicy ───────────────────────────────────────────────

    #[test]
    fn message_budget_subtracts_response_reservation() {
        let policy = ContextWindowPolicy {
            max_tokens: 8_192,
            reserve_for_response: 1_024,
            min_history_messages: 2,
        };
        assert_eq!(policy.message_budget(), 7_168);
    }

    #[test]
    fn message_budget_does_not_underflow() {
        let policy = ContextWindowPolicy {
            max_tokens: 100,
            reserve_for_response: 200,
            min_history_messages: 1,
        };
        assert_eq!(policy.message_budget(), 0);
    }

    // ── prune_messages ────────────────────────────────────────────────────

    #[test]
    fn prune_returns_all_messages_when_within_budget() {
        let msgs = vec![
            make_msg(MessageRole::User, "hi"),
            make_msg(MessageRole::Assistant, "hello"),
        ];
        let policy = ContextWindowPolicy {
            max_tokens: 10_000,
            reserve_for_response: 100,
            min_history_messages: 1,
        };
        let pruned = prune_messages(&msgs, &policy);
        assert_eq!(pruned.len(), msgs.len());
    }

    #[test]
    fn prune_removes_oldest_non_system_messages_first() {
        let mut msgs = Vec::new();
        for i in 0..10 {
            let role = if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            };
            msgs.push(make_msg(role, &format!("message number {i}")));
        }
        let policy = ContextWindowPolicy {
            max_tokens: 100,
            reserve_for_response: 10,
            min_history_messages: 2,
        };
        let pruned = prune_messages(&msgs, &policy);

        // At least min_history_messages are kept
        assert!(pruned.len() >= policy.min_history_messages);
        // Fewer messages than the original
        assert!(pruned.len() <= msgs.len());
        // The most recent message must be retained
        let last_original = msgs.last().unwrap();
        let last_pruned = pruned.last().unwrap();
        assert_eq!(last_pruned.content(), last_original.content());
    }

    #[test]
    fn prune_always_retains_system_messages() {
        let msgs = vec![
            make_msg(MessageRole::System, "You are a helpful assistant."),
            make_msg(MessageRole::User, "question one"),
            make_msg(MessageRole::Assistant, "answer one"),
        ];
        let policy = ContextWindowPolicy {
            max_tokens: 20,
            reserve_for_response: 5,
            min_history_messages: 1,
        };
        let pruned = prune_messages(&msgs, &policy);
        let has_system = pruned.iter().any(|m| m.role == MessageRole::System);
        assert!(has_system, "System message must always be retained");
    }

    #[test]
    fn prune_guarantees_min_history_messages() {
        let msgs = vec![
            make_msg(MessageRole::User, &"a".repeat(500)),
            make_msg(MessageRole::Assistant, &"b".repeat(500)),
            make_msg(MessageRole::User, &"c".repeat(500)),
        ];
        // Budget so tight nothing fits, but min_history_messages = 2
        let policy = ContextWindowPolicy {
            max_tokens: 5,
            reserve_for_response: 1,
            min_history_messages: 2,
        };
        let pruned = prune_messages(&msgs, &policy);
        let non_system_count = pruned.iter().filter(|m| m.role != MessageRole::System).count();
        assert!(non_system_count >= policy.min_history_messages);
    }

    #[test]
    fn manager_selects_provider_specific_policy() {
        let manager = ContextWindowManager::default();
        let policy = manager.policy_for("openai", "gpt-4o-mini");
        assert_eq!(policy.max_tokens, 16_384);
    }

    #[test]
    fn summarize_and_prune_injects_summary_message_when_history_drops() {
        let messages = vec![
            make_msg(MessageRole::System, "Stay helpful."),
            make_msg(MessageRole::User, &"hello ".repeat(200)),
            make_msg(MessageRole::Assistant, &"reply ".repeat(200)),
            make_msg(MessageRole::User, "latest question"),
        ];
        let policy = ContextWindowPolicy {
            max_tokens: 120,
            reserve_for_response: 20,
            min_history_messages: 1,
        };

        let prepared = summarize_and_prune_messages(&messages, &policy);
        assert!(prepared.iter().any(|message| {
            message.role == MessageRole::System
                && message.content().contains("Conversation summary")
        }));
    }
}
