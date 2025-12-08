//! Command detection tests

use antikhitera_mcp_client::tui::screens::chat::ChatState;

#[test]
fn test_is_command_with_slash() {
    let mut state = ChatState::new();
    state.input = "/help".to_string();
    assert!(state.is_command());
}

#[test]
fn test_is_command_with_colon() {
    let mut state = ChatState::new();
    state.input = ":config".to_string();
    assert!(state.is_command());
}

#[test]
fn test_is_not_command() {
    let mut state = ChatState::new();
    state.input = "Hello there".to_string();
    assert!(!state.is_command());
}

#[test]
fn test_get_command() {
    let mut state = ChatState::new();
    state.input = "/agent on".to_string();
    assert_eq!(state.get_command(), Some("agent"));
}

#[test]
fn test_get_command_none() {
    let mut state = ChatState::new();
    state.input = "regular text".to_string();
    assert_eq!(state.get_command(), None);
}
