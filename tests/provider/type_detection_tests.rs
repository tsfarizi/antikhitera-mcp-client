// Provider config tests - testing ModelProviderConfig behavior
//
// Tests for provider type detection and helper methods.
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

fn minimal_model() -> &'static str {
    r#"
default_provider = "test"
model = "model"
prompt_template = "test"
"#
}

#[test]
fn is_ollama_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "test"
type = "OLLAMA"
endpoint = "http://localhost:11434"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model());

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
}

#[test]
fn is_gemini_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "test"
type = "GEMINI"
endpoint = "https://example.com"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model());

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_gemini());
}

#[test]
fn provider_type_mixed_case() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "test"
type = "OlLaMa"
endpoint = "http://localhost:11434"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model());

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
    assert!(!config.providers[0].is_gemini());
}
