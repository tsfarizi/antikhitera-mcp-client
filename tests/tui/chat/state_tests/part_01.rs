#[test]
fn test_chat_state_new() {
    let state = ChatState::new();

    assert!(state.messages.is_empty());
    assert!(state.input.is_empty());
    assert_eq!(state.cursor_pos, 0);
    assert_eq!(state.scroll_offset, 0);
    assert!(state.agent_mode);
    assert!(state.session_id.is_none());
    assert!(!state.loading);
}

