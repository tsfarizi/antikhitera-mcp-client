//! Comprehensive Session Module Tests
//!
//! Extensive test suite for antikythera-session with focus on:
//! - Session creation and lifecycle
//! - Concurrent session management
//! - Message integrity and ordering
//! - Serialization/deserialization roundtrips
//! - Data corruption recovery
//! - Performance under load

use antikythera_session::*;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

// ============================================================================
// MESSAGE CREATION & OPERATIONS
// ============================================================================

#[test]
fn test_message_user_creation() {
    let msg = Message::user("Hello, world!");
    
    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "Hello, world!");
    assert_eq!(msg.tool_name, None);
    assert_eq!(msg.step, None);
}

#[test]
fn test_message_assistant_creation() {
    let msg = Message::assistant("I'm here to help!");
    
    assert_eq!(msg.role, MessageRole::Assistant);
    assert_eq!(msg.content, "I'm here to help!");
}

#[test]
fn test_message_system_creation() {
    let msg = Message::system("System initialized");
    
    assert_eq!(msg.role, MessageRole::System);
    assert_eq!(msg.content, "System initialized");
}

#[test]
fn test_message_tool_result_creation() {
    let args = serde_json::json!({ "city": "NYC" });
    let msg = Message::tool_result("get_weather", "72°F, sunny", Some(args.clone()), 1);
    
    assert_eq!(msg.role, MessageRole::ToolResult);
    assert_eq!(msg.content, "72°F, sunny");
    assert_eq!(msg.tool_name, Some("get_weather".to_string()));
    assert_eq!(msg.step, Some(1));
    assert!(msg.tool_args.is_some());
}

#[test]
fn test_message_with_metadata() {
    let msg = Message::user("test").with_metadata(r#"{"priority": "high"}"#);
    
    assert_eq!(msg.metadata, Some(r#"{"priority": "high"}"#.to_string()));
}

#[test]
fn test_message_empty_content() {
    let msg = Message::user("");
    assert_eq!(msg.content, "");
}

#[test]
fn test_message_very_long_content() {
    let long_content = "x".repeat(1_000_000);
    let msg = Message::user(&long_content);
    assert_eq!(msg.content, long_content);
}

#[test]
fn test_message_unicode_content() {
    let unicode_msgs = vec![
        "こんにちは世界",
        "你好世界",
        "مرحبا بالعالم",
        "🚀 Rust 🦀",
        "Mixed: English + 日本語 + 😀",
    ];
    
    for content in unicode_msgs {
        let msg = Message::user(content);
        assert_eq!(msg.content, content);
    }
}

#[test]
fn test_message_special_characters() {
    let special_contents = vec![
        "Quote: \"hello\"",
        "Newline: hello\nworld",
        "Tab: hello\tworld",
        "Backslash: \\path\\to\\file",
        "JSON: {\"key\": \"value\"}",
        "SQL: DROP TABLE users;",
    ];
    
    for content in special_contents {
        let msg = Message::user(content);
        assert_eq!(msg.content, content);
    }
}

#[test]
fn test_message_role_str_conversion() {
    assert_eq!(MessageRole::User.as_str(), "user");
    assert_eq!(MessageRole::Assistant.as_str(), "assistant");
    assert_eq!(MessageRole::System.as_str(), "system");
    assert_eq!(MessageRole::ToolResult.as_str(), "tool_result");
}

#[test]
fn test_message_tool_result_without_args() {
    let msg = Message::tool_result("get_time", "2024-04-20", None, 42);
    
    assert_eq!(msg.role, MessageRole::ToolResult);
    assert_eq!(msg.tool_name, Some("get_time".to_string()));
    assert_eq!(msg.tool_args, None);
    assert_eq!(msg.step, Some(42));
}

// ============================================================================
// MESSAGE SERIALIZATION
// ============================================================================

#[test]
fn test_message_json_serialization_roundtrip() {
    let original = Message::user("test message");
    let json = original.to_json().unwrap();
    let restored = Message::from_json(&json).unwrap();
    
    assert_eq!(restored.role, original.role);
    assert_eq!(restored.content, original.content);
}

#[test]
fn test_message_json_with_all_fields() {
    let original = Message::tool_result("get_weather", "sunny", Some(serde_json::json!({"city": "NYC"})), 5)
        .with_metadata(r#"{"critical": true}"#);
    
    let json = original.to_json().unwrap();
    let restored = Message::from_json(&json).unwrap();
    
    assert_eq!(restored.role, MessageRole::ToolResult);
    assert_eq!(restored.content, "sunny");
    assert_eq!(restored.tool_name, Some("get_weather".to_string()));
    assert_eq!(restored.step, Some(5));
    assert_eq!(restored.metadata, Some(r#"{"critical": true}"#.to_string()));
}

#[test]
fn test_message_invalid_json_deserialization() {
    let invalid_json = r#"{"role": "invalid", "content": "x"}"#;
    let result = Message::from_json(invalid_json);
    
    assert!(result.is_err());
}

#[test]
fn test_message_json_injection_escape() {
    let injection = r#"","role":"assistant","#;
    let msg = Message::user(injection);
    
    let json = msg.to_json().unwrap();
    let restored = Message::from_json(&json).unwrap();
    
    assert_eq!(restored.content, injection);
}

// ============================================================================
// SESSION CREATION & LIFECYCLE
// ============================================================================

#[test]
fn test_session_creation() {
    let session = Session::new("user-123", "gpt-4");
    
    assert_eq!(session.user_id, "user-123");
    assert_eq!(session.model, "gpt-4");
    assert_eq!(session.messages.len(), 0);
    assert!(!session.id.is_empty()); // Should have generated ID
}

#[test]
fn test_session_with_id() {
    let mut session = Session::new("user-123", "gpt-4");
    session.id = "custom-id-123".to_string();
    
    assert_eq!(session.id, "custom-id-123");
}

#[test]
fn test_session_with_title() {
    let mut session = Session::new("user-123", "gpt-4");
    session.title = Some("Weather Discussion".to_string());
    
    assert_eq!(session.title, Some("Weather Discussion".to_string()));
}

#[test]
fn test_session_add_message() {
    let mut session = Session::new("user-123", "gpt-4");
    
    session.add_message(Message::user("Hello"));
    assert_eq!(session.messages.len(), 1);
    
    session.add_message(Message::assistant("Hi there!"));
    assert_eq!(session.messages.len(), 2);
}

#[test]
fn test_session_message_ordering() {
    let mut session = Session::new("user-123", "gpt-4");
    
    let msg1 = Message::user("First");
    let msg2 = Message::assistant("Second");
    let msg3 = Message::user("Third");
    
    session.add_message(msg1.clone());
    session.add_message(msg2.clone());
    session.add_message(msg3.clone());
    
    assert_eq!(session.messages[0].content, "First");
    assert_eq!(session.messages[1].content, "Second");
    assert_eq!(session.messages[2].content, "Third");
}

#[test]
fn test_session_empty_user_id() {
    let session = Session::new("", "gpt-4");
    assert_eq!(session.user_id, "");
}

#[test]
fn test_session_unicode_model_name() {
    let session = Session::new("user", "GPT-4_日本語_🚀");
    assert_eq!(session.model, "GPT-4_日本語_🚀");
}

// ============================================================================
// SESSION MANAGER - BASIC OPERATIONS
// ============================================================================

#[test]
fn test_session_manager_create_session() {
    let manager = SessionManager::new();
    let session_id = manager.create_session("user-123", "gpt-4");
    
    assert!(!session_id.is_empty());
    assert!(manager.has_session(&session_id));
}

#[test]
fn test_session_manager_create_with_custom_id() {
    let manager = SessionManager::new();
    let custom_id = "my-custom-session-id";
    let session_id = manager.create_session_with_id(custom_id, "user-123", "gpt-4");
    
    assert_eq!(session_id, custom_id);
    assert!(manager.has_session(custom_id));
}

#[test]
fn test_session_manager_get_session() {
    let manager = SessionManager::new();
    let id = manager.create_session("user-123", "gpt-4");
    
    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.user_id, "user-123");
    assert_eq!(session.model, "gpt-4");
}

#[test]
fn test_session_manager_get_nonexistent_session() {
    let manager = SessionManager::new();
    let session = manager.get_session("nonexistent");
    
    assert!(session.is_none());
}

#[test]
fn test_session_manager_add_message() {
    let manager = SessionManager::new();
    let id = manager.create_session("user-123", "gpt-4");
    
    let msg = Message::user("Hello");
    let result = manager.add_message(&id, msg);
    
    assert!(result.is_ok());
    
    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.messages.len(), 1);
}

#[test]
fn test_session_manager_add_message_to_nonexistent() {
    let manager = SessionManager::new();
    let msg = Message::user("Hello");
    
    let result = manager.add_message("nonexistent", msg);
    assert!(result.is_err());
}

#[test]
fn test_session_manager_list_sessions() {
    let manager = SessionManager::new();
    
    manager.create_session("user-1", "gpt-4");
    manager.create_session("user-2", "gpt-3");
    manager.create_session("user-3", "claude");
    
    let summaries = manager.list_sessions();
    assert_eq!(summaries.len(), 3);
}

#[test]
fn test_session_manager_session_count() {
    let manager = SessionManager::new();
    assert_eq!(manager.session_count(), 0);
    
    manager.create_session("user-1", "gpt-4");
    assert_eq!(manager.session_count(), 1);
    
    manager.create_session("user-2", "gpt-3");
    assert_eq!(manager.session_count(), 2);
}

// ============================================================================
// CONCURRENT SESSION OPERATIONS
// ============================================================================

#[test]
fn test_concurrent_session_creation() {
    let manager = Arc::new(SessionManager::new());
    let thread_count = 10;
    let sessions_per_thread = 50;
    
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            for i in 0..sessions_per_thread {
                manager_clone.create_session(
                    &format!("user-{}-{}", thread_id, i),
                    "gpt-4",
                );
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(manager.session_count(), thread_count * sessions_per_thread);
}

#[test]
fn test_concurrent_message_addition() {
    let manager = Arc::new(SessionManager::new());
    let session_id = manager.create_session("user-123", "gpt-4");
    
    let thread_count = 20;
    let messages_per_thread = 50;
    
    let mut handles = vec![];
    
    for thread_id in 0..thread_count {
        let manager_clone = manager.clone();
        let session_id_clone = session_id.clone();
        
        let handle = thread::spawn(move || {
            for msg_id in 0..messages_per_thread {
                let msg = Message::user(&format!("t{}-m{}", thread_id, msg_id));
                manager_clone.add_message(&session_id_clone, msg).ok();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let session = manager.get_session(&session_id).unwrap();
    assert_eq!(session.messages.len(), thread_count * messages_per_thread);
}

#[test]
fn test_concurrent_read_write() {
    let manager = Arc::new(SessionManager::new());
    
    let mut handles = vec![];
    
    // Creator threads
    for i in 0..3 {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            for j in 0..100 {
                manager_clone.create_session(&format!("user-{}-{}", i, j), "gpt-4");
            }
        });
        handles.push(handle);
    }
    
    // Reader threads
    for _ in 0..3 {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            for _ in 0..200 {
                let _ = manager_clone.list_sessions();
                let _ = manager_clone.session_count();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(manager.session_count(), 300);
}

// ============================================================================
// MESSAGE HISTORY INTEGRITY
// ============================================================================

#[test]
fn test_message_history_ordering_on_concurrent_adds() {
    let manager = Arc::new(SessionManager::new());
    let session_id = manager.create_session("user", "gpt-4");
    
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];
    
    for thread_id in 0..10 {
        let manager_clone = manager.clone();
        let session_id_clone = session_id.clone();
        let barrier_clone = barrier.clone();
        
        let handle = thread::spawn(move || {
            barrier_clone.wait(); // Synchronize all threads
            
            for i in 0..10 {
                let msg = Message::user(&format!("t{}-m{}", thread_id, i));
                manager_clone.add_message(&session_id_clone, msg).ok();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let session = manager.get_session(&session_id).unwrap();
    assert_eq!(session.messages.len(), 100);
}

#[test]
fn test_session_message_capacity() {
    let manager = SessionManager::new();
    let id = manager.create_session("user", "gpt-4");
    
    // Add many messages
    for i in 0..10_000 {
        let msg = Message::user(&format!("msg-{}", i));
        manager.add_message(&id, msg).ok();
    }
    
    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.messages.len(), 10_000);
}

// ============================================================================
// SERIALIZATION & EXPORT/IMPORT
// ============================================================================

#[test]
fn test_session_summary_creation() {
    let manager = SessionManager::new();
    let id = manager.create_session("user-123", "gpt-4");
    
    let session = manager.get_session(&id).unwrap();
    let summary = SessionSummary::from(&session);
    
    assert_eq!(summary.id, session.id);
    assert_eq!(summary.user_id, "user-123");
    assert_eq!(summary.model, "gpt-4");
}

#[test]
fn test_session_export_creation() {
    let manager = SessionManager::new();
    let id = manager.create_session("user-123", "gpt-4");
    
    manager.add_message(&id, Message::user("Hello"));
    manager.add_message(&id, Message::assistant("Hi!"));
    
    let session = manager.get_session(&id).unwrap();
    let export = SessionExport::from_session(session);
    
    assert_eq!(export.session.messages.len(), 2);
}

#[test]
fn test_session_export_with_unicode() {
    let mut session = Session::new("user", "gpt-4");
    session.add_message(Message::user("こんにちは"));
    session.add_message(Message::assistant("🌟 Bonjour"));
    
    let export = SessionExport::from_session(session);
    assert_eq!(export.session.messages.len(), 2);
}

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

// ============================================================================
// ERROR HANDLING & EDGE CASES
// ============================================================================

#[test]
fn test_duplicate_session_creation_with_same_id() {
    let manager = SessionManager::new();
    
    let id1 = manager.create_session_with_id("id-123", "user-1", "gpt-4");
    let id2 = manager.create_session_with_id("id-123", "user-2", "gpt-3");
    
    // Second creation should overwrite
    assert_eq!(id1, id2);
    
    let session = manager.get_session("id-123").unwrap();
    assert_eq!(session.user_id, "user-2"); // Latest value
    assert_eq!(session.model, "gpt-3"); // Latest value
}

#[test]
fn test_empty_session_id_handling() {
    let manager = SessionManager::new();
    
    // Some operations with empty session ID
    let result = manager.add_message("", Message::user("test"));
    assert!(result.is_err());
    
    let session = manager.get_session("");
    assert!(session.is_none());
}

#[test]
fn test_very_long_session_id() {
    let manager = SessionManager::new();
    let long_id = "s".repeat(100_000);
    
    let id = manager.create_session_with_id(&long_id, "user", "gpt-4");
    assert_eq!(id, long_id);
    assert!(manager.has_session(&long_id));
}

#[test]
fn test_unicode_session_id() {
    let manager = SessionManager::new();
    let unicode_id = "session-🚀-日本語-العربية";
    
    let id = manager.create_session_with_id(unicode_id, "user", "gpt-4");
    assert_eq!(id, unicode_id);
    assert!(manager.has_session(unicode_id));
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_rapid_session_creation() {
    let manager = SessionManager::new();
    let start = Instant::now();
    
    for i in 0..10_000 {
        manager.create_session(&format!("user-{}", i), "gpt-4");
    }
    
    let elapsed = start.elapsed();
    let per_second = (10_000.0 / elapsed.as_secs_f64()) as u64;
    
    println!("Created 10k sessions in {:?} ({} sess/sec)", elapsed, per_second);
    assert!(elapsed.as_secs() < 10);
}

#[test]
fn test_rapid_message_addition() {
    let manager = SessionManager::new();
    let id = manager.create_session("user", "gpt-4");
    
    let start = Instant::now();
    
    for i in 0..10_000 {
        let msg = Message::user(&format!("msg-{}", i));
        manager.add_message(&id, msg).ok();
    }
    
    let elapsed = start.elapsed();
    let per_second = (10_000.0 / elapsed.as_secs_f64()) as u64;
    
    println!("Added 10k messages in {:?} ({} msg/sec)", elapsed, per_second);
    assert!(elapsed.as_secs() < 10);
}

#[test]
fn test_large_session_retrieval() {
    let manager = SessionManager::new();
    let id = manager.create_session("user", "gpt-4");
    
    for i in 0..5_000 {
        let msg = Message::user(&format!("msg-{}", i));
        manager.add_message(&id, msg).ok();
    }
    
    let start = Instant::now();
    let _session = manager.get_session(&id).unwrap();
    let elapsed = start.elapsed();
    
    println!("Retrieved session with 5k messages in {:?}", elapsed);
    assert!(elapsed.as_millis() < 100);
}

#[test]
fn test_many_sessions_list() {
    let manager = SessionManager::new();
    
    for i in 0..1_000 {
        manager.create_session(&format!("user-{}", i), "gpt-4");
    }
    
    let start = Instant::now();
    let summaries = manager.list_sessions();
    let elapsed = start.elapsed();
    
    assert_eq!(summaries.len(), 1_000);
    println!("Listed 1k sessions in {:?}", elapsed);
    assert!(elapsed.as_millis() < 500);
}
