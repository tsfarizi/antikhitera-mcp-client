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
    let path = write_configs(dir.path(), client_content, minimal_model(), minimal_ui());

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
    let path = write_configs(dir.path(), minimal_client(), model_content, minimal_ui());

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::ProviderNotFound { .. })));
}

// =============================================================================
// Integration tests - load actual config files from config/ directory
// =============================================================================

/// Test that actual config/client.toml and config/model.toml can be loaded
