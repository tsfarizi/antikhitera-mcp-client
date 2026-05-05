use antikythera_core::application::stdio::suggest_commands;

#[test]
fn suggest_commands_returns_defaults_for_empty_prefix() {
    let suggestions = suggest_commands("");
    assert!(!suggestions.is_empty());
    assert!(suggestions.contains(&"help"));
}

#[test]
fn suggest_commands_matches_partial_input() {
    let suggestions = suggest_commands("co");
    assert!(suggestions.iter().any(|value| value.starts_with("config")));
}

#[test]
fn suggest_commands_returns_empty_for_unknown_prefix() {
    let suggestions = suggest_commands("zzzzz");
    assert!(suggestions.is_empty());
}
