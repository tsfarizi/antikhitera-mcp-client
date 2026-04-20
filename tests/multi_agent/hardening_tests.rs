//! Centralized tests for multi-agent production hardening types.
//!
//! These are pure-logic tests that do not require a live LLM or MCP server.
//! They cover: `AgentTask` builder, serde roundtrips, `TaskRetryPolicy`,
//! `TaskExecutionMetadata` defaults, `TaskResult` constructors,
//! `PipelineResult` aggregation, `budget_steps` guardrail semantics, and
//! deadline pre-check logic.

use antikythera_core::application::agent::multi_agent::task::{
    AgentTask, ErrorKind, PipelineResult, RetryCondition, RoutingDecision, TaskExecutionMetadata,
    TaskResult, TaskRetryPolicy,
};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

// Split by concern to keep file size manageable and improve readability.
include!("hardening_tests/part_01.rs");
include!("hardening_tests/part_02.rs");
include!("hardening_tests/part_03.rs");
include!("hardening_tests/part_04.rs");
include!("hardening_tests/part_05.rs");
include!("hardening_tests/part_06.rs");
include!("hardening_tests/part_07.rs");
include!("hardening_tests/part_08.rs");
include!("hardening_tests/part_09.rs");
include!("hardening_tests/part_10.rs");
include!("hardening_tests/part_11.rs");
include!("hardening_tests/part_12.rs");
include!("hardening_tests/part_13.rs");
