// Config loading tests - testing AppConfig::load error handling
//
// Tests focused on configuration file loading and validation errors.

use antikhitera_mcp_client::config::{AppConfig, ConfigError};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("client.toml");
    fs::write(&path, content).expect("Failed to write config");
    path
}

#[test]
fn returns_error_when_file_not_found() {
    let result = AppConfig::load(Some(Path::new("/nonexistent/path/client.toml")));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

#[test]
fn returns_error_when_model_missing() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
default_provider = "gemini"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingModel)));
}

#[test]
fn returns_error_when_default_provider_missing() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
model = "test-model"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingDefaultProvider)));
}

#[test]
fn returns_error_when_prompt_template_missing() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
model = "test-model"
default_provider = "gemini"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingPromptTemplate)));
}

#[test]
fn returns_error_when_no_providers() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
model = "test-model"
default_provider = "gemini"
prompt_template = "test"
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::NoProvidersConfigured)));
}

#[test]
fn returns_error_when_provider_missing_endpoint() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
model = "test-model"
default_provider = "gemini"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
models = ["test"]
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingEndpoint { .. })));
}

#[test]
fn returns_error_when_default_provider_not_in_list() {
    let dir = tempdir().expect("tempdir");
    let path = write_config(
        dir.path(),
        r#"
model = "test-model"
default_provider = "nonexistent"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
    );

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::ProviderNotFound { .. })));
}
