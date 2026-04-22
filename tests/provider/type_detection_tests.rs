// Provider config tests - testing ModelProviderConfig behavior
//
// Tests for provider type detection and helper methods.
// Uses CLI's ModelProviderConfig directly — no file I/O required.

use antikythera_cli::infrastructure::llm::ModelProviderConfig;

// Split into 5 parts for consistent test organization.
include!("type_detection_tests/part_01.rs");
include!("type_detection_tests/part_02.rs");
include!("type_detection_tests/part_03.rs");
include!("type_detection_tests/part_04.rs");
include!("type_detection_tests/part_05.rs");
