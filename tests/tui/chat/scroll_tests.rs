//! Scroll tests

use antikhitera_mcp_client::tui::screens::chat::ChatState;

#[test]
fn test_scroll_up() {
    let mut state = ChatState::new();
    state.scroll_offset = 5;

    state.scroll_up();
    assert_eq!(state.scroll_offset, 4);
}

#[test]
fn test_scroll_down() {
    let mut state = ChatState::new();
    state.scroll_offset = 5;

    state.scroll_down(100);
    assert_eq!(state.scroll_offset, 6);
}

#[test]
fn test_scroll_to_bottom() {
    let mut state = ChatState::new();
    state.scroll_offset = 10;

    state.scroll_to_bottom();
    assert_eq!(state.scroll_offset, u16::MAX);
}

#[test]
fn test_scroll_up_at_zero() {
    let mut state = ChatState::new();
    state.scroll_offset = 0;

    state.scroll_up();
    assert_eq!(state.scroll_offset, 0);
}
