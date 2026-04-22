#[test]
fn detects_provider_types_via_postcard_conversion() {
    let ollama_pc = ProviderConfig {
        id: "ollama".to_string(),
        provider_type: "ollama".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: String::new(),
        models: vec![PostcardModelInfo {
            name: "llama3".to_string(),
            display_name: String::new(),
        }],
    };
    let gemini_pc = ProviderConfig {
        id: "gemini".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: "https://generativelanguage.googleapis.com".to_string(),
        api_key: "secret".to_string(),
        models: vec![PostcardModelInfo {
            name: "gemini-1.5-flash".to_string(),
            display_name: "Gemini Flash".to_string(),
        }],
    };

    let providers: Vec<ModelProviderConfig> =
        providers_from_postcard(&[ollama_pc, gemini_pc]);

    assert_eq!(providers.len(), 2);

    let ollama = providers.iter().find(|p| p.id == "ollama").unwrap();
    assert!(ollama.is_ollama());
    assert!(!ollama.is_gemini());

    let gemini = providers.iter().find(|p| p.id == "gemini").unwrap();
    assert!(gemini.is_gemini());
    assert!(!gemini.is_ollama());
    assert_eq!(gemini.api_key.as_deref(), Some("secret"));
}

