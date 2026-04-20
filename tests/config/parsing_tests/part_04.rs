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

    let path = write_configs(
        dir.path(),
        client_content,
        model_content,
        minimal_ui_config(),
    );
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

