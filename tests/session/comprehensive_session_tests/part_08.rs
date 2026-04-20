// ============================================================================
// CLONE & SHARE BEHAVIOR
// ============================================================================

#[test]
fn test_manager_clone_shares_data() {
    let manager1 = SessionManager::new();
    let id = manager1.create_session("user", "gpt-4");
    
    let manager2 = manager1.clone();
    
    // Both managers should see the same session
    assert!(manager1.has_session(&id));
    assert!(manager2.has_session(&id));
}

#[test]
fn test_session_clone_independence() {
    let session1 = Session::new("user", "gpt-4");
    let mut session2 = session1.clone();
    
    session2.add_message(Message::user("test"));
    
    // session1 should still be empty
    assert_eq!(session1.messages.len(), 0);
    assert_eq!(session2.messages.len(), 1);
}

