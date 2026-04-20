#[test]
fn test_move_cursor_left() {
    let mut state = ChatState::new();
    state.input = "Hello".to_string();
    state.cursor_pos = 3;

    state.move_cursor_left();
    assert_eq!(state.cursor_pos, 2);

    state.move_cursor_left();
    state.move_cursor_left();
    state.move_cursor_left();
    assert_eq!(state.cursor_pos, 0);
}


#[test]
fn test_move_cursor_right() {
    let mut state = ChatState::new();
    state.input = "Hi".to_string();
    state.cursor_pos = 0;

    state.move_cursor_right();
    assert_eq!(state.cursor_pos, 1);

    state.move_cursor_right();
    assert_eq!(state.cursor_pos, 2);

    state.move_cursor_right();
    assert_eq!(state.cursor_pos, 2);
}

