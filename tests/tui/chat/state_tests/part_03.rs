#[test]
fn test_add_message() {
    let mut state = ChatState::new();

    state.add_message(ChatMessage::user("Hello"));
    state.add_message(ChatMessage::assistant("Hi!"));

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, MessageRole::User);
    assert_eq!(state.messages[1].role, MessageRole::Assistant);
}

