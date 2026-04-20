//! Integration Tests with Conditional Execution
//!
//! These tests automatically skip if prerequisites (servers, configs, API keys)
//! are not available. Each skipped test provides clear instructions on how to
//! set up the required dependencies.

mod test_utils;

use test_utils::*;

/// Example test that requires configuration files

// Split into 5 parts for consistent test organization.
include!("integration_tests/part_01.rs");
include!("integration_tests/part_02.rs");
include!("integration_tests/part_03.rs");
include!("integration_tests/part_04.rs");
include!("integration_tests/part_05.rs");
