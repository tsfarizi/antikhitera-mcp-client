//! Input handling tests

use antikhitera_mcp_client::tui::screens::chat::ChatState;

#[test]
fn test_insert_char() {
    let mut state = ChatState::new();

    state.insert_char('H');
    state.insert_char('i');

    assert_eq!(state.input, "Hi");
    assert_eq!(state.cursor_pos, 2);
}

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

#[test]
fn test_delete_char_forward() {
    let mut state = ChatState::new();
    state.input = "Hello".to_string();
    state.cursor_pos = 0;

    state.delete_char_forward();

    assert_eq!(state.input, "ello");
    assert_eq!(state.cursor_pos, 0);
}

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
