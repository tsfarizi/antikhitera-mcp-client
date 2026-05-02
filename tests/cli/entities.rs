use antikythera_cli::domain::entities::{ChatSession, ProviderConfig, ProviderType};

#[test]
fn provider_type_parse_is_case_insensitive() {
    assert_eq!(ProviderType::parse("GeMiNi"), Some(ProviderType::Gemini));
    assert_eq!(ProviderType::parse("OLLAMA"), Some(ProviderType::Ollama));
    assert_eq!(ProviderType::parse("openai"), Some(ProviderType::OpenAi));
    assert_eq!(ProviderType::parse("OPENAI"), Some(ProviderType::OpenAi));
    assert_eq!(ProviderType::parse("unknown"), None);
}

#[test]
fn chat_session_starts_with_defaults() {
    let provider = ProviderConfig {
        id: "p1".to_string(),
        provider_type: ProviderType::Ollama,
        endpoint: "http://127.0.0.1:11434".to_string(),
        api_key: None,
        model: "llama3".to_string(),
    };
    let session = ChatSession::new(provider);
    assert!(session.id.starts_with("session-"));
    assert!(session.messages.is_empty());
    assert!(session.agent_mode);
    assert_eq!(session.max_steps, 10);
    assert_eq!(session.current_step, 0);
}

#[test]
fn chat_session_max_steps_works() {
    let provider = ProviderConfig {
        id: "p1".to_string(),
        provider_type: ProviderType::OpenAi,
        endpoint: "https://api.openai.com".to_string(),
        api_key: Some("ENV_KEY".to_string()),
        model: "gpt-4o".to_string(),
    };
    let mut session = ChatSession::new(provider);
    session.current_step = 10;
    assert!(session.is_max_steps_exceeded());
}
