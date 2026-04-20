#[test]
fn test_is_not_command() {
    let mut state = ChatState::new();
    state.input = "Hello there".to_string();
    assert!(!state.is_command());
}

