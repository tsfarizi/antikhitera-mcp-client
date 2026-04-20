// ============================================================================
// URL & PROTOCOL TESTS
// ============================================================================

#[test]
fn test_https_endpoint() {
    let config = ModelProviderConfig {
        id: "secure".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.starts_with("https://"));
}

#[test]
fn test_http_endpoint() {
    let config = ModelProviderConfig {
        id: "local".to_string(),
        provider_type: "test".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.starts_with("http://"));
}

#[test]
fn test_malformed_url() {
    let config = ModelProviderConfig {
        id: "bad".to_string(),
        provider_type: "test".to_string(),
        endpoint: "not-a-url".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    // Config layer accepts any string; validation is caller's responsibility
    assert_eq!(config.endpoint, "not-a-url");
}

