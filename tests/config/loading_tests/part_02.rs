#[test]
fn returns_error_when_model_missing() {
    let dir = tempdir().expect("tempdir");
    // model.toml without "model" field
    let model_content = r#"
default_provider = "gemini"
prompt_template = "test"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content, minimal_ui());

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
    let path = write_configs(dir.path(), minimal_client(), model_content, minimal_ui());

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::MissingDefaultProvider)));
}

