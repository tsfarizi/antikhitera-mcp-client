#[test]
fn test_get_command_none() {
    let mut state = ChatState::new();
    state.input = "regular text".to_string();
    assert_eq!(state.get_command(), None);
}
