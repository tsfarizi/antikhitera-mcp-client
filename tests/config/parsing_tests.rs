// Config parsing tests - testing successful config parsing
//
// Tests for valid configuration parsing including providers, servers, and tools.
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
prompt_template = "You are a helpful assistant."
"#
}

#[test]
fn parses_minimal_valid_config() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(dir.path(), minimal_client_config(), minimal_model_config());

    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.model, "gemini-1.5-flash");
    assert_eq!(config.default_provider, "gemini");
    assert_eq!(config.prompt_template, "You are a helpful assistant.");
    assert_eq!(config.providers.len(), 1);
}

#[test]
fn parses_multiple_providers() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "ollama"
type = "ollama"
endpoint = "http://localhost:11434"
models = ["llama3"]

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "secret"
models = [{ name = "gemini-1.5-flash", display_name = "Gemini Flash" }]
"#;

    let model_content = r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "test"
"#;

    let path = write_configs(dir.path(), client_content, model_content);
    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.providers.len(), 2);

    let ollama = config.providers.iter().find(|p| p.id == "ollama").unwrap();
    assert!(ollama.is_ollama());
    assert!(!ollama.is_gemini());

    let gemini = config.providers.iter().find(|p| p.id == "gemini").unwrap();
    assert!(gemini.is_gemini());
    assert!(!gemini.is_ollama());
    assert_eq!(gemini.api_key.as_deref(), Some("secret"));
}

#[test]
fn parses_servers_and_tools() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "ollama"
type = "ollama"
endpoint = "http://localhost:11434"
models = ["mistral"]

[[servers]]
name = "time"
command = "server.exe"
args = ["--flag"]
workdir = "C:/work"
default_timezone = "Asia/Jakarta"
default_city = "Jakarta"

[[servers]]
name = "other"
command = "other.exe"
"#;

    let model_content = r#"
model = "mistral"
default_provider = "ollama"
prompt_template = "Be helpful."

[[tools]]
name = "get_time"
description = "Fetch time"
server = "time"
"#;

    let path = write_configs(dir.path(), client_content, model_content);
    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.servers.len(), 2);
    assert_eq!(config.servers[0].name, "time");
    assert_eq!(config.servers[0].args, vec!["--flag"]);
    assert_eq!(
        config.servers[0].default_timezone.as_deref(),
        Some("Asia/Jakarta")
    );

    assert_eq!(config.tools.len(), 1);
    assert_eq!(config.tools[0].name, "get_time");
    assert_eq!(config.tools[0].server.as_deref(), Some("time"));
}
