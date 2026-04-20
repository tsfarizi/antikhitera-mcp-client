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

