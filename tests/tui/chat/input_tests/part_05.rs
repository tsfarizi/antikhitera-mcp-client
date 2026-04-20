#[test]
fn test_move_cursor_home_end() {
    let mut state = ChatState::new();
    state.input = "Hello World".to_string();
    state.cursor_pos = 5;

    state.move_cursor_home();
    assert_eq!(state.cursor_pos, 0);

    state.move_cursor_end();
    assert_eq!(state.cursor_pos, 11);
}


#[test]
fn test_take_input() {
    let mut state = ChatState::new();
    state.input = "Test message".to_string();
    state.cursor_pos = 5;

    let input = state.take_input();

    assert_eq!(input, "Test message");
    assert!(state.input.is_empty());
    assert_eq!(state.cursor_pos, 0);
}
