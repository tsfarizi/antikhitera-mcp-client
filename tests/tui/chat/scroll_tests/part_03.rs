#[test]
fn test_scroll_down() {
    let mut state = ChatState::new();
    state.scroll_offset = 5;

    state.scroll_down(100);
    assert_eq!(state.scroll_offset, 6);
}

