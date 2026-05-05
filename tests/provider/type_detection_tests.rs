// Provider config tests - testing ModelProviderConfig behavior
//
// Tests for provider type detection and helper methods.
// Uses CLI's ModelProviderConfig directly — no file I/O required.

use antikythera_cli::infrastructure::llm::ModelProviderConfig;
use antikythera_core::domain::content::{
    parse_step_output, ContentItem, FileContent, FileMetadata,
};
use antikythera_core::domain::types::{ChatMessage, MessagePart, MessageRole};
use antikythera_core::infrastructure::model::{HostModelResponse, ModelError};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::json;

// Split into 8 parts for consistent test organization.
include!("type_detection_tests/part_01.rs");
include!("type_detection_tests/part_02.rs");
include!("type_detection_tests/part_03.rs");
include!("type_detection_tests/part_04.rs");
include!("type_detection_tests/part_05.rs");
include!("type_detection_tests/part_06.rs");
include!("type_detection_tests/part_07.rs");
include!("type_detection_tests/part_08.rs");
