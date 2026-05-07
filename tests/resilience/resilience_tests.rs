//! Integration tests for the resilience module.
//!
//! Verifies that the public API of `antikythera_core::resilience` works
//! correctly end-to-end from an external crate perspective, mirroring the
//! access pattern a host application would use.

use antikythera_core::domain::types::{ChatMessage, MessageRole};
use antikythera_core::resilience::{
    ContextWindowPolicy, HealthStatus, HealthTracker, InMemoryAuditSink, PolicyAuditEvent,
    PolicyAuditSink, PolicyEventType, ResilienceConfig, ResilienceManager, RetryPolicy,
    TimeoutPolicy, TokenEstimator, prune_messages, with_retry, with_retry_if,
};

// Split into 13 parts for consistent test organization.
include!("resilience_tests/part_01.rs");
include!("resilience_tests/part_02.rs");
include!("resilience_tests/part_03.rs");
include!("resilience_tests/part_04.rs");
include!("resilience_tests/part_05.rs");
include!("resilience_tests/part_06.rs");
include!("resilience_tests/part_07.rs");
include!("resilience_tests/part_08.rs");
include!("resilience_tests/part_09.rs");
include!("resilience_tests/part_10.rs");
include!("resilience_tests/part_11.rs");
