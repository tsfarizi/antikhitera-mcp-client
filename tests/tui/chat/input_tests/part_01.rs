#[test]
fn test_insert_char() {
    let mut state = ChatState::new();

    state.insert_char('H');
    state.insert_char('i');

    assert_eq!(state.input, "Hi");
    assert_eq!(state.cursor_pos, 2);
}

