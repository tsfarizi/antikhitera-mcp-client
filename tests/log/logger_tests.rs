//! Antikythera Log Module Tests
//!
//! Tests for the unified logging system including:
//! - Basic logging operations
//! - Log filtering and pagination
//! - Log serialization
//! - Session-based logging

use antikythera_log::*;

// Split into 5 parts for consistent test organization.
include!("logger_tests/part_01.rs");
include!("logger_tests/part_02.rs");
include!("logger_tests/part_03.rs");
include!("logger_tests/part_04.rs");
include!("logger_tests/part_05.rs");
