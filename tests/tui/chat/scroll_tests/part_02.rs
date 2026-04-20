#[test]
fn test_scroll_up() {
    let mut state = ChatState::new();
    state.scroll_offset = 5;

    state.scroll_up();
    assert_eq!(state.scroll_offset, 4);
}

