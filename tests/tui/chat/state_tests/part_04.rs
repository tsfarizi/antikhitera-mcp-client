#[test]
fn test_reset() {
    let mut state = ChatState::new();
    state.add_message(ChatMessage::user("Test"));
    state.session_id = Some("abc123".to_string());
    state.scroll_offset = 10;

    state.reset();

    assert!(state.messages.is_empty());
    assert!(state.session_id.is_none());
    assert_eq!(state.scroll_offset, 0);
    assert!(state.status_message.is_some());
}

