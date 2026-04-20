#[test]
fn test_get_command() {
    let mut state = ChatState::new();
    state.input = "/agent on".to_string();
    assert_eq!(state.get_command(), Some("agent"));
}

