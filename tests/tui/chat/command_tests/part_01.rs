#[test]
fn test_is_command_with_slash() {
    let mut state = ChatState::new();
    state.input = "/help".to_string();
    assert!(state.is_command());
}

