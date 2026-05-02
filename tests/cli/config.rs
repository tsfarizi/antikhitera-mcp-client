use antikythera_cli::config::{
    AppConfig, config_from_postcard, config_to_postcard, load_app_config, normalize_provider_type,
    recommended_default_config,
};
use std::path::Path;

#[test]
fn roundtrip_postcard_uses_typed_result() {
    let config = AppConfig::default();
    let bytes = config_to_postcard(&config).expect("serialize");
    let decoded = config_from_postcard(&bytes).expect("deserialize");
    assert_eq!(
        decoded.model.default_provider,
        config.model.default_provider
    );
}

#[test]
fn missing_file_returns_typed_error() {
    let missing = Path::new("definitely-not-exists-app.pc");
    let err = load_app_config(Some(missing)).expect_err("missing file should error");
    assert!(err.to_string().contains("configuration error"));
}

#[test]
fn deprecated_aliases_delegate_to_new_names() {
    #[allow(deprecated)]
    use antikythera_cli::config::load_config;
    let missing = Path::new("definitely-not-exists-app.pc");
    let e1 = load_app_config(Some(missing))
        .expect_err("expected error")
        .to_string();
    #[allow(deprecated)]
    let e2 = load_config(Some(missing))
        .expect_err("expected error")
        .to_string();
    assert_eq!(e1, e2);
}

#[test]
fn recommended_default_config_includes_primary_providers() {
    let config = recommended_default_config();
    let ids: Vec<&str> = config.providers.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"gemini"));
    assert!(ids.contains(&"openai"));
    assert!(ids.contains(&"ollama"));
    assert_eq!(config.model.default_provider, "ollama");
}

#[test]
fn normalize_provider_type_maps_known_aliases() {
    assert_eq!(normalize_provider_type("GEMINI"), "gemini");
    assert_eq!(normalize_provider_type("google-ai"), "gemini");
    assert_eq!(normalize_provider_type("LOCALAI"), "ollama");
    assert_eq!(normalize_provider_type("openai"), "openai");
}
