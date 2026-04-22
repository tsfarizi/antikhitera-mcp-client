#[test]
fn is_ollama_case_insensitive() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "OLLAMA".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };
    assert!(config.is_ollama());
}

