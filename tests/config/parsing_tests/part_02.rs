#[test]
fn parses_minimal_valid_config() {
    let dir = tempdir().expect("tempdir");
    let mut pc = minimal_postcard_config();
    pc.prompts.template = "You are a helpful assistant.".to_string();
    let path = write_postcard_config(dir.path(), &pc);

    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.model, "gemini-1.5-flash");
    assert_eq!(config.default_provider, "gemini");
    assert_eq!(config.prompt_template(), "You are a helpful assistant.");
}

