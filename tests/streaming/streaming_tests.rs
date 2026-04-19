use antikythera_core::cli::Cli;
use antikythera_sdk::agents::{StreamingModeOption, StreamingOptions};
use clap::Parser;

#[test]
fn cli_stream_flag_defaults_to_false() {
    let cli = Cli::try_parse_from(["mcp"]).expect("cli parse should succeed");
    assert!(!cli.stream);
}

#[test]
fn cli_stream_flag_can_be_enabled() {
    let cli = Cli::try_parse_from(["mcp", "--stream"]).expect("cli parse should succeed");
    assert!(cli.stream);
}

#[test]
fn sdk_streaming_options_convert_to_core_request() {
    let options = StreamingOptions {
        mode: StreamingModeOption::Event,
        include_final_response: false,
        max_buffered_events: Some(32),
    };

    let request = options.to_streaming_request();
    assert_eq!(request.mode, antikythera_core::StreamingMode::Event);
    assert!(!request.include_final_response);
    assert_eq!(request.max_buffered_events, Some(32));
}

#[test]
fn sdk_streaming_options_validate_positive_buffer() {
    let invalid = StreamingOptions {
        mode: StreamingModeOption::Mixed,
        include_final_response: true,
        max_buffered_events: Some(0),
    };

    let error = invalid
        .validate()
        .expect_err("max_buffered_events=0 should be rejected");
    assert!(error.contains("max_buffered_events"));
}
