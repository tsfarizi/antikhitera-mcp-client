#[test]
fn test_scroll_up_at_zero() {
    let mut state = ChatState::new();
    state.scroll_offset = 0;

    state.scroll_up();
    assert_eq!(state.scroll_offset, 0);
}
