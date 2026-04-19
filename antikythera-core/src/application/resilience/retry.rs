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
use std::future::Future;
use tracing::warn;

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
                warn!(
                    attempt = attempt,
                    max = policy.max_attempts,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "Transient failure — retrying after back-off"
                );
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

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn fast_policy(max: u32) -> RetryPolicy {
        RetryPolicy {
            max_attempts: max,
            initial_delay_ms: 1,
            max_delay_ms: 5,
            backoff_factor: 1.0,
        }
    }

    #[tokio::test]
    async fn succeeds_on_first_attempt_without_retry() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result: Result<&str, String> = with_retry(&RetryPolicy::no_retry(), || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok("ok")
            }
        })
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_up_to_max_attempts_on_every_failure() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result: Result<&str, String> = with_retry(&fast_policy(3), || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("network error".to_string())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stops_immediately_when_predicate_returns_false() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result: Result<&str, String> = with_retry_if(
            &fast_policy(5),
            || {
                let c = Arc::clone(&c);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err("permanent error".to_string())
                }
            },
            |_| false, // never retry
        )
        .await;
        assert!(result.is_err());
        // Only one attempt — predicate prevented any retries
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn succeeds_after_two_transient_failures() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result: Result<u32, String> = with_retry(&fast_policy(5), || {
            let c = Arc::clone(&c);
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("transient".to_string())
                } else {
                    Ok(n)
                }
            }
        })
        .await;
        assert!(result.is_ok());
        // 2 failures + 1 success = 3 total calls
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_predicate_is_checked_per_error() {
        // Only retry on "retry-me"; bail on "permanent"
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result: Result<&str, &str> = with_retry_if(
            &fast_policy(10),
            || {
                let c = Arc::clone(&c);
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    match n {
                        0 => Err("retry-me"),
                        _ => Err("permanent"),
                    }
                }
            },
            |e| *e == "retry-me",
        )
        .await;
        assert_eq!(result.unwrap_err(), "permanent");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
