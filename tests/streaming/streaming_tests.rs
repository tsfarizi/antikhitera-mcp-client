use antikythera_core::cli::Cli;
use antikythera_core::{
    AgentEvent, BufferPolicy, ClientInputStream, InMemoryStreamingResponse, StreamingBuffer,
    StreamingPhase2Options, StreamingRequest, StreamingResponse,
};
use antikythera_sdk::agents::{StreamingModeOption, StreamingOptions};
use clap::Parser;

// Split into 5 parts for consistent test organization.
include!("streaming_tests/part_01.rs");
include!("streaming_tests/part_02.rs");
include!("streaming_tests/part_03.rs");
include!("streaming_tests/part_04.rs");
include!("streaming_tests/part_05.rs");
