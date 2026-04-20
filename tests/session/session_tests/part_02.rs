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

