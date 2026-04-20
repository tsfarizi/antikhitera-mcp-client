#[test]
fn test_message_system() {
    let msg = ChatMessage::system("Welcome");
    assert_eq!(msg.role, MessageRole::System);
    assert_eq!(msg.content, "Welcome");
}
