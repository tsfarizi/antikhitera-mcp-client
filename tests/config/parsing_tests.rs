// Config parsing tests - testing successful config parsing
//
// Tests for valid configuration parsing including providers, servers, and tools.
// Updated to use split config: client.toml + model.toml

use antikythera_core::config::AppConfig;
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

/// Minimal client.toml content
fn minimal_client_config() -> &'static str {
    r#"
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "TEST_KEY"
models = [{ name = "gemini-1.5-flash" }]
"#
}

/// Minimal model.toml content
fn minimal_model_config() -> &'static str {
    r#"
default_provider = "gemini"
model = "gemini-1.5-flash"

[prompts]
template = "You are a helpful assistant."
"#
}

/// Minimal ui.toml content
fn minimal_ui_config() -> &'static str {
    r#"
[components.text]
required_fields = ["content"]
field_types = { content = "string" }
is_container = false
"#
}

// Split into 5 parts for consistent test organization.
include!("parsing_tests/part_01.rs");
include!("parsing_tests/part_02.rs");
include!("parsing_tests/part_03.rs");
include!("parsing_tests/part_04.rs");
include!("parsing_tests/part_05.rs");
