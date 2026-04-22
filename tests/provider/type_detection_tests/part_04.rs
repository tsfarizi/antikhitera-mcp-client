#[test]
fn is_gemini_case_insensitive() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "GEMINI".to_string(),
        endpoint: "https://example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };
    assert!(config.is_gemini());
}

