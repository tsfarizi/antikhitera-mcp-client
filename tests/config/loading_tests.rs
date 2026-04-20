// Config loading tests - testing AppConfig::load error handling
//
// Tests focused on configuration file loading and validation errors.
// Updated to use split config: client.toml + model.toml

use antikythera_core::config::{AppConfig, ConfigError};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Write client.toml, model.toml, and ui.toml to the temp directory
fn write_configs(
    dir: &Path,
    client_content: &str,
    model_content: &str,
    ui_content: &str,
) -> std::path::PathBuf {
    let client_path = dir.join("client.toml");
    let model_path = dir.join("model.toml");
    let ui_path = dir.join("ui.toml");
    fs::write(&client_path, client_content).expect("Failed to write client.toml");
    fs::write(&model_path, model_content).expect("Failed to write model.toml");
    fs::write(&ui_path, ui_content).expect("Failed to write ui.toml");
    client_path
}

fn minimal_ui() -> &'static str {
    r#"
[components.text]
required_fields = ["content"]
field_types = { content = "string" }
is_container = false
"#
}

fn minimal_client() -> &'static str {
    r#"
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#
}

fn minimal_model() -> &'static str {
    r#"
default_provider = "gemini"
model = "test-model"
prompt_template = "test"
"#
}

// Split into 5 parts for consistent test organization.
include!("loading_tests/part_01.rs");
include!("loading_tests/part_02.rs");
include!("loading_tests/part_03.rs");
include!("loading_tests/part_04.rs");
include!("loading_tests/part_05.rs");
