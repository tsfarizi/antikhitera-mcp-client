// Serialization tests - testing config TOML serialization
//
// Tests for converting AppConfig back to TOML format.

use antikhitera_mcp_client::config::AppConfig;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("client.toml");
    fs::write(&path, content).expect("Failed to write config");
    path
}

fn minimal_valid_config() -> &'static str {
    r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "You are a helpful assistant."

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "TEST_KEY"
models = [{ name = "gemini-1.5-flash" }]
"#
}

#[test]
fn to_raw_toml_contains_required_fields() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(dir.path(), minimal_valid_config());

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
    let path = write_config(dir.path(), minimal_valid_config());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("id = \"gemini\""));
    assert!(raw.contains("type = \"gemini\""));
    assert!(raw.contains("endpoint = \"https://generativelanguage.googleapis.com\""));
}

#[test]
fn to_raw_toml_handles_system_prompt() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "Template"
system_prompt = "Be helpful and concise."

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["gemini-1.5-flash"]
"#,
    );

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("system_prompt = \"Be helpful and concise.\""));
}
