#[test]
fn test_message_assistant() {
    let msg = ChatMessage::assistant("Hi there!");
    assert_eq!(msg.role, MessageRole::Assistant);
    assert_eq!(msg.content, "Hi there!");
}

