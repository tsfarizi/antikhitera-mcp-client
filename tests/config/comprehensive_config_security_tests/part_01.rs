// ============================================================================
// MODEL PROVIDER CONFIG TESTS
// ============================================================================

#[test]
fn test_model_provider_config_basic() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("secret".to_string()),
        api_path: Some("/v1beta".to_string()),
        models: vec![ModelInfo {
            name: "model-1".to_string(),
            display_name: Some("Model 1".to_string()),
        }],
    };

    assert_eq!(config.id, "test");
    assert_eq!(config.provider_type, "gemini");
    assert_eq!(config.models.len(), 1);
}

#[test]
fn test_model_provider_config_no_api_key() {
    let config = ModelProviderConfig {
        id: "ollama-local".to_string(),
        provider_type: "ollama".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.api_key, None);
}

#[test]
fn test_model_provider_config_multiple_models() {
    let config = ModelProviderConfig {
        id: "openai".to_string(),
        provider_type: "openai".to_string(),
        endpoint: "https://api.openai.com".to_string(),
        api_key: Some("key".to_string()),
        api_path: None,
        models: vec![
            ModelInfo { name: "gpt-4".to_string(), display_name: None },
            ModelInfo { name: "gpt-3.5".to_string(), display_name: Some("GPT-3.5 Turbo".to_string()) },
        ],
    };

    assert_eq!(config.models.len(), 2);
}

#[test]
fn test_model_provider_config_unicode_id() {
    let config = ModelProviderConfig {
        id: "\u{63d0}\u{4f9b}\u{8005}_\u{1f680}".to_string(),
        provider_type: "custom".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.id, "\u{63d0}\u{4f9b}\u{8005}_\u{1f680}");
}

#[test]
fn test_model_provider_config_unicode_endpoint() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com/\u{6a21}\u{578b}".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.contains("\u{6a21}\u{578b}"));
}

#[test]
fn test_model_provider_config_very_long_id() {
    let long_id = "x".repeat(10_000);
    let config = ModelProviderConfig {
        id: long_id.clone(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.id.len(), 10_000);
}

#[test]
fn test_model_provider_config_very_long_api_key() {
    let long_key = "k".repeat(1_000_000);
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some(long_key.clone()),
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.api_key.as_ref().unwrap().len(), 1_000_000);
}

#[test]
fn test_model_provider_config_very_long_endpoint() {
    let long_url = format!("https://api.example.com/{}", "x".repeat(100_000));
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: long_url.clone(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.endpoint.len(), "https://api.example.com/".len() + 100_000);
}

#[test]
fn test_model_provider_config_clone() {
    let original = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("secret".to_string()),
        api_path: Some("/v1".to_string()),
        models: vec![],
    };

    let cloned = original.clone();
    assert_eq!(original, cloned);
}

