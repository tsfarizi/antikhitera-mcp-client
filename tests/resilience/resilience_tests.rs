//! Integration tests for the resilience module.
//!
//! Verifies that the public API of `antikythera_core::resilience` works
//! correctly end-to-end from an external crate perspective, mirroring the
//! access pattern a host application would use.

use antikythera_core::domain::types::{ChatMessage, MessageRole};
use antikythera_core::resilience::{
    ContextWindowPolicy, HealthStatus, HealthTracker, ResilienceConfig, ResilienceManager,
    RetryPolicy, TimeoutPolicy, TokenEstimator, prune_messages, with_retry_if,
};

// â”€â”€ RetryPolicy integration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// Split into 5 parts for consistent test organization.
include!("resilience_tests/part_01.rs");
include!("resilience_tests/part_02.rs");
include!("resilience_tests/part_03.rs");
include!("resilience_tests/part_04.rs");
include!("resilience_tests/part_05.rs");
