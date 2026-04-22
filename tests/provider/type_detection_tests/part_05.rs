#[test]
fn provider_type_mixed_case() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "OlLaMa".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };
    assert!(config.is_ollama());
    assert!(!config.is_gemini());
}
