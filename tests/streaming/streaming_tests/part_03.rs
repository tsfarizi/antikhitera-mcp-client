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

// Phase 2 integration tests

