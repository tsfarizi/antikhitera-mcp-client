//! Cooperative task cancellation for the multi-agent orchestrator.
//!
//! [`CancellationToken`] is a lightweight, cloneable handle backed by a
//! shared `Arc<AtomicBool>`.  Multiple tasks can hold child tokens derived
//! from the same root; cancelling the root cancels all children simultaneously.
//!
//! # Example
//!
//! ```rust
//! use antikythera_core::application::agent::multi_agent::cancellation::CancellationToken;
//!
//! let token = CancellationToken::new();
//! let child = token.child_token();
//!
//! assert!(!token.is_cancelled());
//! token.cancel();
//! assert!(token.is_cancelled());
//! assert!(child.is_cancelled(), "child shares the same flag");
//! ```

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};

// ============================================================================
// CancellationToken
// ============================================================================

/// A cloneable cancellation signal.
///
/// All tokens created via [`child_token`] share the same underlying flag, so
/// cancelling one cancels all.  This is intentional — the orchestrator owns
/// the root token and distributes child tokens to individual task executors.
///
/// [`child_token`]: CancellationToken::child_token
#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Create a new, non-cancelled token.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Signal cancellation.  All tokens sharing this flag will observe
    /// [`is_cancelled`] returning `true` after this call.
    ///
    /// [`is_cancelled`]: CancellationToken::is_cancelled
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if [`cancel`] has been called.
    ///
    /// [`cancel`]: CancellationToken::cancel
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Returns a new token that shares the same underlying cancellation flag.
    pub fn child_token(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

// ============================================================================
// CancellationHandle (serialisable snapshot for introspection)
// ============================================================================

/// A serialisable snapshot of the cancellation state, suitable for embedding
/// in [`TaskExecutionMetadata`].
///
/// [`TaskExecutionMetadata`]: super::task::TaskExecutionMetadata
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CancellationSnapshot {
    /// Whether the task was cancelled externally before or during execution.
    pub was_cancelled: bool,
}

impl From<&CancellationToken> for CancellationSnapshot {
    fn from(t: &CancellationToken) -> Self {
        Self { was_cancelled: t.is_cancelled() }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_token_is_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_sets_flag() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn child_token_shares_flag() {
        let root = CancellationToken::new();
        let child = root.child_token();
        assert!(!child.is_cancelled());
        root.cancel();
        assert!(child.is_cancelled(), "child must observe parent cancellation");
    }

    #[test]
    fn clone_shares_flag() {
        let token = CancellationToken::new();
        let cloned = token.clone();
        token.cancel();
        assert!(cloned.is_cancelled());
    }

    #[test]
    fn cancellation_snapshot_reflects_state() {
        let token = CancellationToken::new();
        let snap = CancellationSnapshot::from(&token);
        assert!(!snap.was_cancelled);
        token.cancel();
        let snap2 = CancellationSnapshot::from(&token);
        assert!(snap2.was_cancelled);
    }
}
