//! Comprehensive Logging Module Tests
//!
//! Extensive test suite for antikythera-log with focus on:
//! - Edge cases and boundary conditions
//! - Concurrency safety and race conditions
//! - Security: input validation, injection prevention
//! - Performance: memory leaks, bounds
//! - Panic safety: no unwrap/expect in hot paths
//! - Data integrity: serialization, ordering

use antikythera_log::*;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

// Split by concern to keep file size manageable and improve readability.
include!("comprehensive_logging_tests/part_01.rs");
include!("comprehensive_logging_tests/part_02.rs");
include!("comprehensive_logging_tests/part_03.rs");
include!("comprehensive_logging_tests/part_04.rs");
include!("comprehensive_logging_tests/part_05.rs");
include!("comprehensive_logging_tests/part_06.rs");
include!("comprehensive_logging_tests/part_07.rs");
include!("comprehensive_logging_tests/part_08.rs");
