#[test]
fn test_is_command_with_colon() {
    let mut state = ChatState::new();
    state.input = ":config".to_string();
    assert!(state.is_command());
}

