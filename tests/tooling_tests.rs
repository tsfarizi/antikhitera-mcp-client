// Tooling tests - verifying high-level tooling functions
//
// Tests for spawn_and_list_tools with different transport types.

use antikythera_core::application::tooling::spawn_and_list_tools;
use antikythera_core::config::{ServerConfig, TransportType};
use std::collections::HashMap;

// Split into 5 parts for consistent test organization.
include!("tooling_tests/part_01.rs");
include!("tooling_tests/part_02.rs");
include!("tooling_tests/part_03.rs");
include!("tooling_tests/part_04.rs");
include!("tooling_tests/part_05.rs");
