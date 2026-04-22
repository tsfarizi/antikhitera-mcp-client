#[test]
fn self_heals_when_postcard_data_is_corrupt() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("app.pc");
    fs::write(&path, b"not valid postcard data").expect("write");

    // Corrupt data triggers self-heal — returns a fresh default rather than an error.
    let config = AppConfig::load(Some(&path)).expect("self-heal should succeed");
    assert!(!config.model.is_empty());
    assert!(!config.default_provider.is_empty());
}

#[test]
fn loads_routing_strings_from_postcard() {
    let dir = tempdir().expect("tempdir");
    let mut pc = minimal_postcard_config();
    pc.model.default_provider = "gemini".to_string();
    pc.model.model = "gemini-1.5-flash".to_string();
    let path = write_postcard_config(dir.path(), &pc);

    let config = AppConfig::load(Some(&path)).expect("load config");
    assert_eq!(config.default_provider, "gemini");
    assert_eq!(config.model, "gemini-1.5-flash");
}

