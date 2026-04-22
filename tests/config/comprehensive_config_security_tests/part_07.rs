// ============================================================================
// STRESS & PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_many_providers_in_config() {
    for i in 0..1000 {
        let config = ProviderConfig {
            id: format!("provider-{}", i),
            provider_type: "test".to_string(),
            endpoint: format!("https://api.example.com/provider-{}", i),
            api_key: format!("key-{}", i),
            models: vec![],
        };

        assert_eq!(config.id, format!("provider-{}", i));
    }
}

#[test]
fn test_many_servers_in_config() {
    for i in 0..1000 {
        let config = ServerConfig {
            name: format!("server-{}", i),
            transport: if i % 2 == 0 { TransportType::Stdio } else { TransportType::Http },
            command: Some(PathBuf::from(format!("/bin/server-{}", i))),
            args: vec![],
            env: HashMap::new(),
            workdir: None,
            url: Some(format!("http://localhost:{}", 3000 + i)),
            headers: HashMap::new(),
            default_timezone: None,
            default_city: None,
        };

        assert_eq!(config.name, format!("server-{}", i));
    }
}

#[test]
fn test_large_model_list() {
    let mut models = vec![];
    for i in 0..10_000 {
        models.push(PcModelInfo {
            name: format!("model-{}", i),
            display_name: format!("Model {}", i),
        });
    }

    let config = ProviderConfig {
        id: "massive".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: String::new(),
        models,
    };

    assert_eq!(config.models.len(), 10_000);
}
