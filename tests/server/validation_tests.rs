// Server validation tests - validating server configuration
//
// Tests that verify configuration references are valid.
// These tests gracefully skip if config files don't exist.

use antikythera_core::config::AppConfig;
use std::path::Path;

// Split into 5 parts for consistent test organization.
include!("validation_tests/part_01.rs");
include!("validation_tests/part_02.rs");
include!("validation_tests/part_03.rs");
include!("validation_tests/part_04.rs");
include!("validation_tests/part_05.rs");
