#[test]
fn returns_error_when_client_file_not_found() {
    let result = AppConfig::load(Some(Path::new("/nonexistent/path/client.toml")));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}


#[test]
fn returns_error_when_model_file_not_found() {
    let dir = tempdir().expect("tempdir");
    // Only write client.toml, not model.toml
    let client_path = dir.path().join("client.toml");
    fs::write(&client_path, minimal_client()).expect("Failed to write");

    let result = AppConfig::load(Some(&client_path));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

