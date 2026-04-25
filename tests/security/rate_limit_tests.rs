//! Rate limiting tests

use antikythera_core::security::config::RateLimitConfig;
use antikythera_core::security::rate_limit::{RateLimiter, RateLimitError};

#[test]
fn test_rate_limiter_creation() {
    let config = RateLimitConfig::default();
    let limiter = RateLimiter::new(config);
    assert_eq!(limiter.config().enabled, true);
}

#[test]
fn test_rate_limit_check_within_limits() {
    let limiter = RateLimiter::from_config();
    let session_id = "test-session-1";

    for _ in 0..10 {
        assert!(limiter.check(session_id).is_ok());
    }
}

#[test]
fn test_rate_limit_exceeded() {
    let config = RateLimitConfig {
        enabled: true,
        requests_per_minute: 3,
        requests_per_hour: 100,
        requests_per_day: 1000,
        max_concurrent_sessions: 10,
        window_size_secs: 60,
        burst_allowance: 0,
        cleanup_interval_secs: 300,
    };
    let limiter = RateLimiter::new(config);
    let session_id = "test-session-2";

    // First 3 requests should succeed
    for _ in 0..3 {
        assert!(limiter.check(session_id).is_ok());
    }

    // 4th request should fail
    assert!(matches!(limiter.check(session_id), Err(RateLimitError::LimitExceeded { .. })));
}

#[test]
fn test_rate_limit_disabled() {
    let config = RateLimitConfig {
        enabled: false,
        ..Default::default()
    };
    let limiter = RateLimiter::new(config);
    let session_id = "test-session-3";

    // Should allow unlimited requests when disabled
    for _ in 0..1000 {
        assert!(limiter.check(session_id).is_ok());
    }
}

#[test]
fn test_get_usage() {
    let limiter = RateLimiter::from_config();
    let session_id = "test-session-4";

    limiter.check(session_id).unwrap();
    limiter.check(session_id).unwrap();

    let usage = limiter.get_usage(session_id);
    assert!(usage.is_some());
    assert_eq!(usage.unwrap().requests_per_minute, 2);
}

#[test]
fn test_reset_session() {
    let limiter = RateLimiter::from_config();
    let session_id = "test-session-5";

    limiter.check(session_id).unwrap();
    limiter.check(session_id).unwrap();

    limiter.reset_session(session_id);

    let usage = limiter.get_usage(session_id);
    assert_eq!(usage.unwrap().requests_per_minute, 0);
}

#[test]
fn test_remove_session() {
    let limiter = RateLimiter::from_config();
    let session_id = "test-session-6";

    limiter.check(session_id).unwrap();
    assert_eq!(limiter.active_session_count(), 1);

    limiter.remove_session(session_id);
    assert_eq!(limiter.active_session_count(), 0);
}

#[test]
fn test_concurrent_sessions_limit() {
    let config = RateLimitConfig {
        enabled: true,
        requests_per_minute: 100,
        requests_per_hour: 1000,
        requests_per_day: 10000,
        max_concurrent_sessions: 2,
        window_size_secs: 60,
        burst_allowance: 0,
        cleanup_interval_secs: 300,
    };
    let limiter = RateLimiter::new(config);

    // Create 2 sessions
    limiter.check("session-1").unwrap();
    limiter.check("session-2").unwrap();

    // 3rd session should fail
    assert!(matches!(
        limiter.check("session-3"),
        Err(RateLimitError::TooManyConcurrentSessions { .. })
    ));
}

#[test]
fn test_update_config() {
    let mut limiter = RateLimiter::from_config();

    let new_config = RateLimitConfig {
        enabled: false,
        requests_per_minute: 1000,
        ..Default::default()
    };

    limiter.update_config(new_config);
    assert_eq!(limiter.config().enabled, false);
    assert_eq!(limiter.config().requests_per_minute, 1000);
}