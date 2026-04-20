#[test]
fn to_raw_toml_includes_provider_details() {
    let dir = tempdir().expect("tempdir");
    let path = write_configs(dir.path(), minimal_client(), minimal_model(), minimal_ui());

    let config = AppConfig::load(Some(&path)).expect("load config");
    let raw = config.to_raw_toml();

    assert!(raw.contains("id = \"gemini\""));
    assert!(raw.contains("type = \"gemini\""));
    assert!(raw.contains("endpoint = \"https://generativelanguage.googleapis.com\""));
}

