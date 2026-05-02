use antikythera_cli::infrastructure::llm::ModelProviderConfig;
use antikythera_cli::runtime::{
    default_provider_template, detect_provider_from_env, materialize_runtime_config,
};
use antikythera_core::AppConfig;
use serial_test::serial;

fn sample_config() -> AppConfig {
    AppConfig {
        default_provider: "ollama".to_string(),
        model: "llama3.2".to_string(),
        system_prompt: None,
        tools: Vec::new(),
        servers: Vec::new(),
        rest_server: Default::default(),
        prompts: Default::default(),
    }
}

fn sample_providers() -> Vec<ModelProviderConfig> {
    vec![default_provider_template("ollama").expect("ollama template")]
}

#[test]
fn materialize_runtime_config_can_auto_add_gemini_template() {
    let (runtime, providers) = materialize_runtime_config(
        &sample_config(),
        &sample_providers(),
        Some("gemini"),
        Some("gemini-2.0-flash"),
        None,
        None,
        None,
    )
    .expect("runtime config");
    assert_eq!(runtime.default_provider, "gemini");
    assert_eq!(runtime.model, "gemini-2.0-flash");
    assert!(providers.iter().any(|p| p.id == "gemini"));
}

#[test]
fn materialize_runtime_config_applies_selected_endpoint_override() {
    let (_, providers) = materialize_runtime_config(
        &sample_config(),
        &sample_providers(),
        Some("openai"),
        Some("gpt-4o-mini"),
        Some("https://example-openai-proxy.test"),
        None,
        None,
    )
    .expect("runtime config");
    let provider = providers
        .iter()
        .find(|p| p.id == "openai")
        .expect("openai provider present");
    assert_eq!(provider.endpoint, "https://example-openai-proxy.test");
}

#[test]
#[serial]
fn detect_provider_from_env_falls_back_to_ollama_when_no_keys() {
    unsafe {
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
    }
    assert_eq!(detect_provider_from_env(), "ollama");
}

#[test]
#[serial]
fn detect_provider_from_env_prefers_gemini_when_key_present() {
    unsafe {
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("GEMINI_API_KEY", "test-key");
    }
    let result = detect_provider_from_env();
    unsafe {
        std::env::remove_var("GEMINI_API_KEY");
    }
    assert_eq!(result, "gemini");
}

#[test]
#[serial]
fn detect_provider_from_env_uses_openai_when_only_openai_key_present() {
    unsafe {
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "test-openai-key");
    }
    let result = detect_provider_from_env();
    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
    assert_eq!(result, "openai");
}
