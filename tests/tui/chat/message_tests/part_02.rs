#[test]
fn test_message_user() {
    let msg = ChatMessage::user("Hello");
    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "Hello");
}

