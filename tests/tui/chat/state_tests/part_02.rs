#[test]
fn test_chat_state_default() {
    let state = ChatState::default();
    assert!(state.messages.is_empty());
    assert!(state.agent_mode);
}

