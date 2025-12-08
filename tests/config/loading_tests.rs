// Config loading tests - testing AppConfig::load error handling
//
// Tests focused on configuration file loading and validation errors.
// Updated to use split config: client.toml + model.toml

use antikhitera_mcp_client::config::{AppConfig, ConfigError};
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

#[test]
fn returns_error_when_client_file_not_found() {
    let result = AppConfig::load(Some(Path::new("/nonexistent/path/client.toml")));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

#[test]
fn returns_error_when_model_file_not_found() {
    let dir = tempdir().expect("tempdir");
    // Only write client.toml, not model.toml
    let client_path = dir.path().join("client.toml");
    fs::write(&client_path, minimal_client()).expect("Failed to write");

    let result = AppConfig::load(Some(&client_path));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

#[test]
fn returns_error_when_model_missing() {
    let dir = tempdir().expect("tempdir");
    // model.toml without "model" field
    let model_content = r#"
default_provider = "gemini"
prompt_template = "test"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content);

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingModel)));
}

#[test]
fn returns_error_when_default_provider_missing() {
    let dir = tempdir().expect("tempdir");
    // model.toml without "default_provider" field
    let model_content = r#"
model = "test-model"
prompt_template = "test"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content);

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingDefaultProvider)));
}

#[test]
fn returns_error_when_prompt_template_missing() {
    let dir = tempdir().expect("tempdir");
    // model.toml without "prompt_template" field
    let model_content = r#"
model = "test-model"
default_provider = "gemini"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content);

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingPromptTemplate)));
}

#[test]
fn returns_error_when_no_providers() {
    let dir = tempdir().expect("tempdir");
    // Empty client.toml (no providers)
    let client_content = r#""#;
    let path = write_configs(dir.path(), client_content, minimal_model());

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::NoProvidersConfigured)));
}

#[test]
fn returns_error_when_provider_missing_endpoint() {
    let dir = tempdir().expect("tempdir");
    // client.toml with provider missing endpoint
    let client_content = r#"
[[providers]]
id = "gemini"
type = "gemini"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model());

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingEndpoint { .. })));
}

#[test]
fn returns_error_when_default_provider_not_in_list() {
    let dir = tempdir().expect("tempdir");
    // model.toml references provider not in client.toml
    let model_content = r#"
model = "test-model"
default_provider = "nonexistent"
prompt_template = "test"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content);

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::ProviderNotFound { .. })));
}
