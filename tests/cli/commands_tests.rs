use antikythera_cli::presentation::tui::handlers::commands::{
    find_provider, render_config_snapshot, render_provider_catalog, resolve_provider_selection,
};
use antikythera_cli::infrastructure::llm::types::ModelInfo;
use antikythera_cli::infrastructure::llm::ModelProviderConfig;
use antikythera_core::application::client::ClientConfigSnapshot;

fn make_provider(id: &str, models: &[&str]) -> ModelProviderConfig {
    ModelProviderConfig {
        id: id.to_string(),
        provider_type: "test".to_string(),
        endpoint: "http://localhost".to_string(),
        api_key: None,
        api_path: None,
        models: models
            .iter()
            .map(|m| ModelInfo {
                name: m.to_string(),
                display_name: None,
            })
            .collect(),
    }
}

// ── find_provider ────────────────────────────────────────────────────
#[test]
fn find_provider_matches_exact() {
    let providers = vec![
        make_provider("gemini", &["gemini-pro"]),
        make_provider("ollama", &["llama3"]),
    ];
    assert!(find_provider(&providers, "gemini").is_some());
    assert!(find_provider(&providers, "ollama").is_some());
    assert!(find_provider(&providers, "nonexistent").is_none());
}

#[test]
fn find_provider_case_insensitive() {
    let providers = vec![make_provider("Gemini", &["gemini-pro"])];
    assert!(find_provider(&providers, "GEMINI").is_some());
    assert!(find_provider(&providers, "gemini").is_some());
    assert!(find_provider(&providers, "  GeMiNi  ").is_some());
}

// ── resolve_provider_selection ───────────────────────────────────────
#[test]
fn resolve_existing_provider_with_provided_model() {
    let providers = vec![make_provider("gemini", &["gemini-pro", "gemini-flash"])];
    let result = resolve_provider_selection(
        &providers,
        "gemini",
        "gemini-pro",
        "gemini",
        Some("gemini-flash"),
    );
    assert_eq!(
        result,
        Ok(("gemini".to_string(), "gemini-flash".to_string()))
    );
}

#[test]
fn resolve_uses_current_model_when_provider_same_and_no_model_input() {
    let providers = vec![make_provider("gemini", &["gemini-pro", "gemini-flash"])];
    let result =
        resolve_provider_selection(&providers, "gemini", "gemini-flash", "gemini", None);
    assert_eq!(
        result,
        Ok(("gemini".to_string(), "gemini-flash".to_string()))
    );
}

#[test]
fn resolve_falls_back_to_first_model() {
    let providers = vec![make_provider("gemini", &["gemini-pro", "gemini-flash"])];
    let result = resolve_provider_selection(&providers, "ollama", "llama3", "gemini", None);
    assert_eq!(result, Ok(("gemini".to_string(), "gemini-pro".to_string())));
}

#[test]
fn resolve_errors_on_unknown_provider() {
    let providers = vec![make_provider("gemini", &["gemini-pro"])];
    let result =
        resolve_provider_selection(&providers, "gemini", "gemini-pro", "unknown", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}

#[test]
fn resolve_errors_when_no_models() {
    let providers = vec![make_provider("empty", &[])];
    let result = resolve_provider_selection(&providers, "other", "", "empty", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("belum memiliki model default"));
}

// ── render_provider_catalog ──────────────────────────────────────────
#[test]
fn render_catalog_includes_provider_ids() {
    let providers = vec![
        make_provider("gemini", &["gemini-pro"]),
        make_provider("ollama", &["llama3"]),
    ];
    let out = render_provider_catalog(&providers, "gemini", "gemini-pro");
    assert!(out.contains("gemini"));
    assert!(out.contains("ollama"));
}

#[test]
fn render_catalog_marks_active() {
    let providers = vec![make_provider("gemini", &["gemini-pro"])];
    let out = render_provider_catalog(&providers, "gemini", "gemini-pro");
    assert!(out.contains("(aktif)"));
}

// ── render_config_snapshot ───────────────────────────────────────────
#[test]
fn render_snapshot_contains_keys() {
    let snap = ClientConfigSnapshot {
        model: "gpt-4".to_string(),
        default_provider: "openai".to_string(),
        system_prompt: Some("You are helpful.".to_string()),
        prompt_template: "{{template}}".to_string(),
        tools: vec![],
        servers: vec![],
        raw: String::new(),
    };
    let out = render_config_snapshot(&snap);
    assert!(out.contains("gpt-4"));
    assert!(out.contains("openai"));
    assert!(out.contains("You are helpful."));
}

#[test]
fn render_snapshot_none_system_prompt() {
    let snap = ClientConfigSnapshot {
        model: "gpt-4".to_string(),
        default_provider: "openai".to_string(),
        system_prompt: None,
        prompt_template: String::new(),
        tools: vec![],
        servers: vec![],
        raw: String::new(),
    };
    let out = render_config_snapshot(&snap);
    assert!(out.contains("<none>"));
}

// ── process_command parsing (name extraction logic) ──────────────────
fn parse_command_name(input: &str) -> String {
    let command = input.trim_start_matches('/').trim();
    let mut parts = command.split_whitespace();
    parts.next().unwrap_or_default().to_ascii_lowercase()
}

#[test]
fn parse_valid_commands() {
    assert_eq!(parse_command_name("/help"), "help");
    assert_eq!(parse_command_name("/model gpt-4"), "model");
    assert_eq!(parse_command_name("/config  "), "config");
    assert_eq!(parse_command_name("/use openai gpt-4o-mini"), "use");
    assert_eq!(parse_command_name("/agent on"), "agent");
    assert_eq!(parse_command_name("/tools"), "tools");
    assert_eq!(parse_command_name("/providers"), "providers");
    assert_eq!(parse_command_name("/reset"), "reset");
    assert_eq!(parse_command_name("/history"), "history");
    assert_eq!(parse_command_name("/exit"), "exit");
}

#[test]
fn parse_edge_cases() {
    assert_eq!(parse_command_name(""), "");
    assert_eq!(parse_command_name("/"), "");
    assert_eq!(parse_command_name("   "), "");
    assert_eq!(parse_command_name("/ unknown"), "unknown");
}
