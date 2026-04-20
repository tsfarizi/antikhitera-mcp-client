#[test]
fn to_raw_toml_contains_required_fields() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(dir.path(), minimal_client(), minimal_model(), minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("default_provider = \"gemini\""));
    assert!(raw.contains("model = \"gemini-1.5-flash\""));
    assert!(raw.contains("[[providers]]"));
    assert!(raw.contains("prompt_template"));
}

