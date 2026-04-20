#[test]
fn test_delete_char_forward() {
    let mut state = ChatState::new();
    state.input = "Hello".to_string();
    state.cursor_pos = 0;

    state.delete_char_forward();

    assert_eq!(state.input, "ello");
    assert_eq!(state.cursor_pos, 0);
}

