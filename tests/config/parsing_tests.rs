// Config parsing tests - testing postcard serialization and provider conversion.
//
// The application stores all configuration as a single Postcard binary (app.pc).
// These tests verify that the postcard roundtrip preserves field values correctly
// and that CLI provider type helpers work as expected.

use antikythera_cli::config::{ModelInfo as PostcardModelInfo, ProviderConfig};
use antikythera_cli::infrastructure::llm::{ModelProviderConfig, providers_from_postcard};
use antikythera_core::config::AppConfig;
use antikythera_core::config::postcard_config::{
    ModelConfig, PostcardAppConfig, config_to_postcard,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Serialize a `PostcardAppConfig` to a temp file and return the path.
fn write_postcard_config(dir: &Path, config: &PostcardAppConfig) -> std::path::PathBuf {
    let path = dir.join("app.pc");
    let data = config_to_postcard(config).expect("Failed to serialize PostcardAppConfig");
    fs::write(&path, &data).expect("Failed to write app.pc");
    path
}

/// A minimal valid `PostcardAppConfig` with sensible defaults for testing.
fn minimal_postcard_config() -> PostcardAppConfig {
    PostcardAppConfig {
        model: ModelConfig {
            default_provider: "gemini".to_string(),
            model: "gemini-1.5-flash".to_string(),
        },
        ..Default::default()
    }
}

// Split into 5 parts for consistent test organization.
include!("parsing_tests/part_01.rs");
include!("parsing_tests/part_02.rs");
include!("parsing_tests/part_03.rs");
include!("parsing_tests/part_04.rs");
include!("parsing_tests/part_05.rs");
