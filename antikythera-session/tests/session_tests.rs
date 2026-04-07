//! Session Manager Tests

use antikythera_session::*;

#[test]
fn test_create_session() {
    let manager = SessionManager::new();

    let session_id = manager.create_session("user-123", "gpt-4");
    assert!(!session_id.is_empty());
    assert!(manager.has_session(&session_id));
}

#[test]
fn test_add_message() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");

    manager
        .add_message(&session_id, Message::user("Hello!"))
        .unwrap();
    manager
        .add_message(&session_id, Message::assistant("Hi there!"))
        .unwrap();

    let history = manager.get_chat_history(&session_id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "Hello!");
    assert_eq!(history[1].content, "Hi there!");
}

#[test]
fn test_list_sessions() {
    let manager = SessionManager::new();

    manager.create_session("user-1", "gpt-4");
    manager.create_session("user-2", "gpt-3.5");

    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 2);
}

#[test]
fn test_delete_session() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");

    assert!(manager.has_session(&session_id));
    manager.delete_session(&session_id).unwrap();
    assert!(!manager.has_session(&session_id));
}

#[test]
fn test_clear_session() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");

    manager.add_message(&session_id, Message::user("Hello!")).unwrap();
    manager.add_message(&session_id, Message::assistant("Hi!")).unwrap();

    let history = manager.get_chat_history(&session_id).unwrap();
    assert_eq!(history.len(), 2);

    manager.clear_session(&session_id).unwrap();
    let history = manager.get_chat_history(&session_id).unwrap();
    assert_eq!(history.len(), 0);
}

#[test]
fn test_export_import_postcard() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");

    manager.add_message(&session_id, Message::user("Test")).unwrap();
    manager.add_message(&session_id, Message::assistant("Response")).unwrap();

    // Export
    let session = manager.get_session(&session_id).unwrap();
    let export = SessionExport::from_session(session);
    let postcard_data = export.to_postcard().unwrap();

    // Import
    let imported = SessionExport::from_postcard(&postcard_data).unwrap();
    assert_eq!(imported.session.user_id, "user-123");
    assert_eq!(imported.session.model, "gpt-4");
    assert_eq!(imported.session.messages.len(), 2);
}

#[test]
fn test_batch_export_import() {
    let manager = SessionManager::new();
    let session1 = manager.create_session("user-1", "gpt-4");
    let session2 = manager.create_session("user-2", "gpt-3.5");

    manager.add_message(&session1, Message::user("Hello")).unwrap();
    manager.add_message(&session2, Message::user("Hi")).unwrap();

    // Export batch
    let sessions: Vec<_> = [
        manager.get_session(&session1).unwrap(),
        manager.get_session(&session2).unwrap(),
    ]
    .to_vec();

    let batch = BatchExport::from_sessions(sessions);
    assert_eq!(batch.session_count(), 2);

    let postcard_data = batch.to_postcard().unwrap();

    // Import batch
    let imported = BatchExport::from_postcard(&postcard_data).unwrap();
    assert_eq!(imported.session_count(), 2);
}

#[test]
fn test_search_sessions() {
    let manager = SessionManager::new();

    let id1 = manager.create_session("user-1", "gpt-4");
    manager.update_title(&id1, "Weather query").unwrap();

    let id2 = manager.create_session("user-1", "gpt-4");
    manager.update_title(&id2, "Code review").unwrap();

    let results = manager.search_sessions("weather");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, Some("Weather query".to_string()));
}

#[test]
fn test_get_sessions_by_user() {
    let manager = SessionManager::new();

    manager.create_session("alice", "gpt-4");
    manager.create_session("alice", "gpt-3.5");
    manager.create_session("bob", "gpt-4");

    let alice_sessions = manager.get_sessions_by_user("alice");
    assert_eq!(alice_sessions.len(), 2);

    let bob_sessions = manager.get_sessions_by_user("bob");
    assert_eq!(bob_sessions.len(), 1);
}

#[test]
fn test_message_serialization() {
    let msg = Message::user("Test message").with_metadata(r#"{"key": "value"}"#);
    let json = msg.to_json().unwrap();
    let restored = Message::from_json(&json).unwrap();

    assert_eq!(restored.content, "Test message");
    assert_eq!(restored.metadata, Some(r#"{"key": "value"}"#.to_string()));
}

#[test]
fn test_session_summary() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");

    manager.add_message(&session_id, Message::user("Hello")).unwrap();
    manager.record_tool(&session_id, "get_weather", 1).unwrap();

    let summary = manager.get_session(&session_id).map(|s| SessionSummary::from(&s));
    let summary = summary.unwrap();

    assert_eq!(summary.message_count, 1);
    assert_eq!(summary.total_steps, 1);
    assert_eq!(summary.tools_used, vec!["get_weather"]);
}
