use antikythera_cli::cli::Cli;
use antikythera_core::{
    AgentEvent, AgentEventStream, BufferPolicy, ClientInputStream, InMemoryStreamingResponse,
    StreamingBuffer, StreamingMode, StreamingPhase2Options, StreamingRequest, StreamingResponse,
    ToolEventPhase,
};
use antikythera_sdk::agents::{StreamingModeOption, StreamingOptions};
use clap::Parser;

// Split into 5 parts for consistent test organization.
include!("streaming_tests/part_01.rs");
include!("streaming_tests/part_02.rs");
include!("streaming_tests/part_03.rs");
include!("streaming_tests/part_04.rs");
include!("streaming_tests/part_05.rs");
include!("streaming_tests/part_06.rs");
