// ============================================================================
// EDGE CASE & BOUNDARY TESTS
// ============================================================================

#[test]
fn test_empty_provider_id() {
    let config = ProviderConfig {
        id: "".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: String::new(),
        models: vec![],
    };

    assert_eq!(config.id, "");
}

#[test]
fn test_empty_server_name() {
    let config = ServerConfig {
        name: "".to_string(),
        transport: TransportType::Stdio,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.name, "");
}

#[test]
fn test_model_provider_with_no_models() {
    let config = ProviderConfig {
        id: "empty".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: String::new(),
        models: vec![],
    };

    assert_eq!(config.models.len(), 0);
}

#[test]
fn test_model_info_without_display_name() {
    let model = PcModelInfo {
        name: "model".to_string(),
        display_name: String::new(),
    };

    assert!(model.display_name.is_empty());
}

