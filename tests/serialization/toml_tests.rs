// Serialization tests - testing config TOML serialization
//
// Tests for converting AppConfig back to TOML format.
// Updated to use split config: client.toml + model.toml

use antikhitera_mcp_client::config::AppConfig;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Write both client.toml and model.toml to the temp directory
fn write_configs(dir: &Path, client_content: &str, model_content: &str) -> std::path::PathBuf {
    let client_path = dir.join("client.toml");
    let model_path = dir.join("model.toml");
    fs::write(&client_path, client_content).expect("Failed to write client.toml");
    fs::write(&model_path, model_content).expect("Failed to write model.toml");
    client_path
}

fn minimal_client() -> &'static str {
    r#"
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "TEST_KEY"
models = [{ name = "gemini-1.5-flash" }]
"#
}

fn minimal_model() -> &'static str {
    r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "You are a helpful assistant."
"#
}

#[test]
fn to_raw_toml_contains_required_fields() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(dir.path(), minimal_client(), minimal_model());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("default_provider = \"gemini\""));
    assert!(raw.contains("model = \"gemini-1.5-flash\""));
    assert!(raw.contains("[[providers]]"));
    assert!(raw.contains("prompt_template"));
}

#[test]
fn to_raw_toml_includes_provider_details() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(dir.path(), minimal_client(), minimal_model());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("id = \"gemini\""));
    assert!(raw.contains("type = \"gemini\""));
    assert!(raw.contains("endpoint = \"https://generativelanguage.googleapis.com\""));
}

#[test]
fn to_raw_toml_handles_system_prompt() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["gemini-1.5-flash"]
"#;
    let model_content = r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "Template"
system_prompt = "Be helpful and concise."
"#;
    let path = write_configs(dir.path(), client_content, model_content);

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("system_prompt = \"Be helpful and concise.\""));
}
