#[test]
fn parses_minimal_valid_config() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(
        dir.path(),
        minimal_client_config(),
        minimal_model_config(),
        minimal_ui_config(),
    );

    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.model, "gemini-1.5-flash");
    assert_eq!(config.default_provider, "gemini");
    assert_eq!(config.prompt_template(), "You are a helpful assistant.");
}

