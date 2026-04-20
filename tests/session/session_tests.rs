//! Antikythera Session Module Tests
//!
//! Tests for session management including:
//! - Session creation and deletion
//! - Message handling
//! - Session export/import
//! - Batch operations

use antikythera_session::*;

// Split into 5 parts for consistent test organization.
include!("session_tests/part_01.rs");
include!("session_tests/part_02.rs");
include!("session_tests/part_03.rs");
include!("session_tests/part_04.rs");
include!("session_tests/part_05.rs");
