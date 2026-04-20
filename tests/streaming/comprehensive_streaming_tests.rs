//! Comprehensive Streaming Module Tests
//!
//! Extensive test suite for antikythera-core streaming with focus on:
//! - Buffer capacity and overflow handling
//! - Event ordering and consistency
//! - Panic safety under load
//! - Edge cases and boundary conditions
//! - Concurrent event generation
//! - Performance under pressure

use antikythera_core::{
    AgentEvent, AgentEventStream, BufferPolicy, ClientInputStream, StreamingBuffer, StreamingMode,
    StreamingRequest, ToolEventPhase,
};
use std::sync::Arc;
use std::thread;

// Split by concern to keep file size manageable and improve readability.
include!("comprehensive_streaming_tests/part_01.rs");
include!("comprehensive_streaming_tests/part_02.rs");
include!("comprehensive_streaming_tests/part_03.rs");
include!("comprehensive_streaming_tests/part_04.rs");
include!("comprehensive_streaming_tests/part_05.rs");
include!("comprehensive_streaming_tests/part_06.rs");
include!("comprehensive_streaming_tests/part_07.rs");
include!("comprehensive_streaming_tests/part_08.rs");
include!("comprehensive_streaming_tests/part_09.rs");
include!("comprehensive_streaming_tests/part_10.rs");
