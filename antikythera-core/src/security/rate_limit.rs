//! Rate Limiting
//!
//! Configurable rate limiting with multiple time windows and burst allowance.

use super::config::RateLimitConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limiter with configurable parameters
pub struct RateLimiter {
    config: RateLimitConfig,
    session_limits: Arc<Mutex<HashMap<String, SessionLimits>>>,
    cleanup_task: Option<std::thread::JoinHandle<()>>,
}

/// Session-specific rate limits
#[derive(Debug, Clone)]
struct SessionLimits {
    minute_window: TimeWindow,
    hour_window: TimeWindow,
    day_window: TimeWindow,
    last_activity: Instant,
}

/// Time window for tracking requests
#[derive(Debug, Clone)]
struct TimeWindow {
    requests: Vec<Instant>,
    window_size: Duration,
    max_requests: u32,
    burst_allowance: u32,
}

impl TimeWindow {
    fn new(window_size_secs: u32, max_requests: u32, burst_allowance: u32) -> Self {
        Self {
            requests: Vec::new(),
            window_size: Duration::from_secs(window_size_secs as u64),
            max_requests,
            burst_allowance,
        }
    }

    fn check(&mut self) -> Result<(), RateLimitError> {
        let now = Instant::now();

        // Remove old requests outside the window
        self.requests
            .retain(|&timestamp| now.duration_since(timestamp) < self.window_size);

        // Check if limit exceeded
        let effective_limit = self.max_requests + self.burst_allowance;
        if self.requests.len() as u32 >= effective_limit {
            return Err(RateLimitError::LimitExceeded {
                limit: self.max_requests,
                current: self.requests.len() as u32,
                window_secs: self.window_size.as_secs(),
            });
        }

        // Add current request
        self.requests.push(now);
        Ok(())
    }

    fn reset(&mut self) {
        self.requests.clear();
    }

    fn request_count(&self) -> u32 {
        self.requests.len() as u32
    }
}

/// Rate limit error
#[derive(Debug, Clone)]
pub enum RateLimitError {
    LimitExceeded {
        limit: u32,
        current: u32,
        window_secs: u64,
    },
    TooManyConcurrentSessions {
        max: u32,
        current: u32,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::LimitExceeded {
                limit,
                current,
                window_secs,
            } => {
                write!(
                    f,
                    "Rate limit exceeded: {}/{} requests per {}s",
                    current, limit, window_secs
                )
            }
            RateLimitError::TooManyConcurrentSessions { max, current } => {
                write!(f, "Too many concurrent sessions: {}/{}", current, max)
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let session_limits = Arc::new(Mutex::new(HashMap::new()));

        let cleanup_task = if config.enabled {
            let limits_clone = Arc::clone(&session_limits);
            let cleanup_interval = Duration::from_secs(config.cleanup_interval_secs as u64);

            Some(std::thread::spawn(move || {
                Self::cleanup_task(limits_clone, cleanup_interval);
            }))
        } else {
            None
        };

        Self {
            config,
            session_limits,
            cleanup_task,
        }
    }

    pub fn from_config() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if a request is allowed for a session
    pub fn check(&self, session_id: &str) -> Result<(), RateLimitError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut limits = self.session_limits.lock().unwrap();

        // Check concurrent session limit
        if limits.len() as u32 >= self.config.max_concurrent_sessions
            && !limits.contains_key(session_id)
        {
            return Err(RateLimitError::TooManyConcurrentSessions {
                max: self.config.max_concurrent_sessions,
                current: limits.len() as u32,
            });
        }

        // Get or create session limits
        let session = limits
            .entry(session_id.to_string())
            .or_insert_with(|| SessionLimits {
                minute_window: TimeWindow::new(
                    60,
                    self.config.requests_per_minute,
                    self.config.burst_allowance,
                ),
                hour_window: TimeWindow::new(
                    3600,
                    self.config.requests_per_hour,
                    self.config.burst_allowance,
                ),
                day_window: TimeWindow::new(
                    86400,
                    self.config.requests_per_day,
                    self.config.burst_allowance,
                ),
                last_activity: Instant::now(),
            });

        session.last_activity = Instant::now();

        // Check all time windows
        session.minute_window.check()?;
        session.hour_window.check()?;
        session.day_window.check()?;

        Ok(())
    }

    /// Get current usage statistics for a session
    pub fn get_usage(&self, session_id: &str) -> Option<SessionUsage> {
        let limits = self.session_limits.lock().unwrap();
        limits.get(session_id).map(|session| SessionUsage {
            requests_per_minute: session.minute_window.request_count(),
            requests_per_hour: session.hour_window.request_count(),
            requests_per_day: session.day_window.request_count(),
            last_activity: session.last_activity,
        })
    }

    /// Reset rate limits for a session
    pub fn reset_session(&self, session_id: &str) {
        let mut limits = self.session_limits.lock().unwrap();
        if let Some(session) = limits.get_mut(session_id) {
            session.minute_window.reset();
            session.hour_window.reset();
            session.day_window.reset();
        }
    }

    /// Remove a session
    pub fn remove_session(&self, session_id: &str) {
        let mut limits = self.session_limits.lock().unwrap();
        limits.remove(session_id);
    }

    /// Get total number of active sessions
    pub fn active_session_count(&self) -> usize {
        let limits = self.session_limits.lock().unwrap();
        limits.len()
    }

    /// Cleanup task to remove inactive sessions
    fn cleanup_task(limits: Arc<Mutex<HashMap<String, SessionLimits>>>, interval: Duration) {
        loop {
            std::thread::sleep(interval);

            let mut limits_guard = limits.lock().unwrap();
            let now = Instant::now();
            let timeout = Duration::from_secs(300); // 5 minutes inactivity timeout

            limits_guard.retain(|_, session| now.duration_since(session.last_activity) < timeout);
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RateLimitConfig) {
        let cleanup_interval_secs = config.cleanup_interval_secs;
        self.config = config;

        // Restart cleanup task if enabled
        if self.config.enabled && self.cleanup_task.is_none() {
            let limits_clone = Arc::clone(&self.session_limits);
            let cleanup_interval = Duration::from_secs(cleanup_interval_secs as u64);

            self.cleanup_task = Some(std::thread::spawn(move || {
                Self::cleanup_task(limits_clone, cleanup_interval);
            }));
        }
    }
}

impl Drop for RateLimiter {
    fn drop(&mut self) {
        // Note: Thread cleanup is handled automatically when the thread completes
        // We don't explicitly abort threads as it can cause resource leaks
        self.cleanup_task.take();
    }
}

/// Session usage statistics
#[derive(Debug, Clone)]
pub struct SessionUsage {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub last_activity: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_check() {
        let limiter = RateLimiter::from_config();
        let session_id = "test-session";

        // Should allow requests within limit
        for _ in 0..10 {
            assert!(limiter.check(session_id).is_ok());
        }
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_minute: 5,
            requests_per_hour: 100,
            requests_per_day: 1000,
            max_concurrent_sessions: 10,
            window_size_secs: 60,
            burst_allowance: 0,
            cleanup_interval_secs: 300,
        };
        let limiter = RateLimiter::new(config);
        let session_id = "test-session";

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(limiter.check(session_id).is_ok());
        }

        // 6th request should fail
        assert!(matches!(
            limiter.check(session_id),
            Err(RateLimitError::LimitExceeded { .. })
        ));
    }

    #[test]
    fn test_get_usage() {
        let limiter = RateLimiter::from_config();
        let session_id = "test-session";

        limiter.check(session_id).unwrap();
        limiter.check(session_id).unwrap();

        let usage = limiter.get_usage(session_id);
        assert!(usage.is_some());
        assert_eq!(usage.unwrap().requests_per_minute, 2);
    }

    #[test]
    fn test_reset_session() {
        let limiter = RateLimiter::from_config();
        let session_id = "test-session";

        limiter.check(session_id).unwrap();
        limiter.reset_session(session_id);

        let usage = limiter.get_usage(session_id);
        assert_eq!(usage.unwrap().requests_per_minute, 0);
    }

    #[test]
    fn test_remove_session() {
        let limiter = RateLimiter::from_config();
        let session_id = "test-session";

        limiter.check(session_id).unwrap();
        assert_eq!(limiter.active_session_count(), 1);

        limiter.remove_session(session_id);
        assert_eq!(limiter.active_session_count(), 0);
    }
}
