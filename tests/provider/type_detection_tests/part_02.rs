#[test]
fn is_ollama_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "test"
type = "OLLAMA"
endpoint = "http://localhost:11434"
models = ["test"]
"#;
    let path = write_configs(dir.path(), client_content, minimal_model(), minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert!(config.providers[0].is_ollama());
}

