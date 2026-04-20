#[test]
fn uses_default_template_when_prompts_missing() {
    let dir = tempdir().expect("tempdir");
    // model.toml without "[prompts]" section
    let model_content = r#"
model = "test-model"
default_provider = "gemini"
"#;
    let path = write_configs(dir.path(), minimal_client(), model_content, minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("should load with default template");
    assert!(
        !config.prompt_template().is_empty(),
        "default template should not be empty"
    );
}


#[test]
fn returns_error_when_no_providers() {
    let dir = tempdir().expect("tempdir");
    // Empty client.toml (no providers)
    let client_content = r#""#;
    let path = write_configs(dir.path(), client_content, minimal_model(), minimal_ui());

    let result = AppConfig::load(Some(&path));
    assert!(matches!(result, Err(ConfigError::NoProvidersConfigured)));
}

