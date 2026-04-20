#[test]
fn test_delete_char() {
    let mut state = ChatState::new();
    state.input = "Hello".to_string();
    state.cursor_pos = 5;

    state.delete_char();

    assert_eq!(state.input, "Hell");
    assert_eq!(state.cursor_pos, 4);
}


#[test]
fn test_delete_char_at_start() {
    let mut state = ChatState::new();
    state.input = "Hello".to_string();
    state.cursor_pos = 0;

    state.delete_char();

    assert_eq!(state.input, "Hello");
    assert_eq!(state.cursor_pos, 0);
}

