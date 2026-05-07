//! Async retry executor with exponential back-off.
//!
//! Provides [`with_retry_if`] — a composable helper that wraps any async
//! fallible call and re-invokes it according to a [`RetryPolicy`] whenever
//! the caller-supplied predicate deems the error transient.
//!
//! # Quick start
//!
//! ```no_run,ignore
//! use antikythera_core::application::resilience::{RetryPolicy, with_retry_if};
//!
//! async fn resilient_call() -> Result<String, String> {
//!     let policy = RetryPolicy::default();
//!     with_retry_if(&policy, || async { Ok("response".to_string()) }, |_| true).await
//! }
//! ```

use super::policy::RetryPolicy;
use crate::logging::ResilienceLogger;
use std::future::Future;

// ── Core executor ─────────────────────────────────────────────────────────────

/// Execute `f` up to `policy.max_attempts` times, retrying only when
/// `should_retry` returns `true` for the error.
///
/// Back-off sleep is driven by `tokio::time::sleep`; the `time` feature must
/// be enabled for the `tokio` dependency (it is, as part of the workspace
/// defaults).
///
/// # Behaviour
///
/// | Outcome                               | Action                    |
/// |---------------------------------------|---------------------------|
/// | `Ok(v)`                               | Return immediately        |
/// | `Err(e)`, attempts < max, should_retry | Sleep then retry          |
/// | `Err(e)`, attempts >= max             | Return `Err(e)`           |
/// | `Err(e)`, `should_retry` → `false`   | Return `Err(e)` now       |
pub async fn with_retry_if<F, Fut, T, E, SR>(
    policy: &RetryPolicy,
    mut f: F,
    should_retry: SR,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    SR: Fn(&E) -> bool,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                attempt += 1;
                if attempt >= policy.max_attempts || !should_retry(&err) {
                    return Err(err);
                }
                let delay = policy.delay_for_attempt(attempt - 1);
                ResilienceLogger::new(&crate::logging::get_active_session()).warn(format!(
                    "Transient failure — retrying after back-off | attempt={} max={} delay_ms={} error={}",
                    attempt, policy.max_attempts, delay.as_millis(), err
                ));
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Execute `f` up to `policy.max_attempts` times, retrying on *every* error.
///
/// This is a convenience wrapper around [`with_retry_if`] that always returns
/// `true` from the retry predicate.
pub async fn with_retry<F, Fut, T, E>(policy: &RetryPolicy, f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    with_retry_if(policy, f, |_| true).await
}
