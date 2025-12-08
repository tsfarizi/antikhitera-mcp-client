// Provider config tests - testing ModelProviderConfig behavior
//
// Tests for provider type detection and helper methods.

use antikhitera_mcp_client::config::AppConfig;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("client.toml");
    fs::write(&path, content).expect("Failed to write config");
    path
}

#[test]
fn is_ollama_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
default_provider = "test"
model = "model"
prompt_template = "test"

[[providers]]
id = "test"
type = "OLLAMA"
endpoint = "http://localhost:11434"
models = ["test"]
"#,
    );

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
}

#[test]
fn is_gemini_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
default_provider = "test"
model = "model"
prompt_template = "test"

[[providers]]
id = "test"
type = "GEMINI"
endpoint = "https://example.com"
models = ["test"]
"#,
    );

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_gemini());
}

#[test]
fn provider_type_mixed_case() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
default_provider = "test"
model = "model"
prompt_template = "test"

[[providers]]
id = "test"
type = "OlLaMa"
endpoint = "http://localhost:11434"
models = ["test"]
"#,
    );

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
    assert!(!config.providers[0].is_gemini());
}
