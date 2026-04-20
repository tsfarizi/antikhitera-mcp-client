#[test]
fn test_scroll_to_bottom() {
    let mut state = ChatState::new();
    state.scroll_offset = 10;

    state.scroll_to_bottom();
    assert_eq!(state.scroll_offset, u16::MAX);
}

