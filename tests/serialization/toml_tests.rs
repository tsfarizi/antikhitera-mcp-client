// Serialization tests - testing config TOML serialization
//
// Tests for converting AppConfig back to TOML format.

use antikythera_core::config::AppConfig;
use antikythera_core::domain::sanitize::{needs_sanitization, sanitize_for_toml};

// Split into 5 parts for consistent test organization.
include!("toml_tests/part_01.rs");
include!("toml_tests/part_02.rs");
include!("toml_tests/part_03.rs");
include!("toml_tests/part_04.rs");
include!("toml_tests/part_05.rs");
include!("toml_tests/part_06.rs");
