//! Context-window management.
//!
//! Provides:
//!
//! - [`TokenEstimator`] — lightweight heuristic token counter (no tokenizer
//!   dependency).
//! - [`ContextWindowPolicy`] — configurable token budget with a response
//!   reservation.
//! - [`prune_messages`] — removes the oldest non-system messages until the
//!   message list fits within the policy budget.
//!
//! # Token estimation accuracy
//!
//! The estimator uses the widely-cited rule of thumb **1 token ≈ 4 characters**
//! for English text.  Accuracy is ±30 % for typical prompts — sufficient for
//! proactive pruning without an ML tokenizer dependency.

use crate::domain::types::{ChatMessage, MessagePart, MessageRole};
#[allow(unused_imports)]
use crate::logging::ResilienceLogger;
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
