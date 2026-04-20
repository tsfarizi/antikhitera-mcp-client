#[test]
fn provider_type_mixed_case() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "test"
type = "OlLaMa"
endpoint = "http://localhost:11434"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model(), minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
    assert!(!config.providers[0].is_gemini());
}
