#[test]
fn to_raw_toml_handles_system_prompt() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["gemini-1.5-flash"]
"#;
    let model_content = r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "Template"
system_prompt = "Be helpful and concise."
"#;
    let path = write_configs(dir.path(), client_content, model_content, minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("system_prompt = \"Be helpful and concise.\""));
}
