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

// Split by concern to keep file size manageable and improve readability.
include!("comprehensive_session_tests/part_01.rs");
include!("comprehensive_session_tests/part_02.rs");
include!("comprehensive_session_tests/part_03.rs");
include!("comprehensive_session_tests/part_04.rs");
include!("comprehensive_session_tests/part_05.rs");
include!("comprehensive_session_tests/part_06.rs");
include!("comprehensive_session_tests/part_07.rs");
include!("comprehensive_session_tests/part_08.rs");
include!("comprehensive_session_tests/part_09.rs");
include!("comprehensive_session_tests/part_10.rs");
