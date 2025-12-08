//! ChatState tests

use antikhitera_mcp_client::tui::screens::chat::{ChatMessage, ChatState, MessageRole};

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

#[test]
fn test_chat_state_default() {
    let state = ChatState::default();
    assert!(state.messages.is_empty());
    assert!(state.agent_mode);
}

#[test]
fn test_add_message() {
    let mut state = ChatState::new();

    state.add_message(ChatMessage::user("Hello"));
    state.add_message(ChatMessage::assistant("Hi!"));

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, MessageRole::User);
    assert_eq!(state.messages[1].role, MessageRole::Assistant);
}

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

#[test]
fn test_toggle_agent_mode() {
    let mut state = ChatState::new();
    assert!(state.agent_mode);

    state.toggle_agent_mode();
    assert!(!state.agent_mode);

    state.toggle_agent_mode();
    assert!(state.agent_mode);
}

#[test]
fn test_loading_tick() {
    let mut state = ChatState::new();
    state.loading = true;
    state.loading_frame = 0;

    state.tick_loading();
    assert_eq!(state.loading_frame, 1);

    state.loading_frame = 3;
    state.tick_loading();
    assert_eq!(state.loading_frame, 0);
}
