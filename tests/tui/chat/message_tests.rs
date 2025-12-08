//! ChatMessage tests

use antikhitera_mcp_client::tui::screens::chat::{ChatMessage, MessageRole};

#[test]
fn test_message_user() {
    let msg = ChatMessage::user("Hello");
    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "Hello");
}

#[test]
fn test_message_assistant() {
    let msg = ChatMessage::assistant("Hi there!");
    assert_eq!(msg.role, MessageRole::Assistant);
    assert_eq!(msg.content, "Hi there!");
}

#[test]
fn test_message_system() {
    let msg = ChatMessage::system("Welcome");
    assert_eq!(msg.role, MessageRole::System);
    assert_eq!(msg.content, "Welcome");
}
