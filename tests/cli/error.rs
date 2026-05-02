use antikythera_cli::error::{CliError, CliResult};

#[test]
fn config_error_display_includes_message() {
    let err = CliError::Config("file not found".to_string());
    assert_eq!(err.to_string(), "configuration error: file not found");
}

#[test]
fn validation_error_display_includes_message() {
    let err = CliError::Validation("bad input".to_string());
    assert_eq!(err.to_string(), "validation error: bad input");
}

#[test]
fn unsupported_error_display_includes_message() {
    let err = CliError::Unsupported("direct LLM call".to_string());
    assert_eq!(err.to_string(), "unsupported operation: direct LLM call");
}

#[test]
fn io_error_is_convertible_from_std_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
    let cli_err: CliError = io_err.into();
    assert!(cli_err.to_string().starts_with("io error:"));
}

#[test]
fn serialization_error_is_convertible_from_serde_json() {
    let serde_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
    let cli_err: CliError = serde_err.into();
    assert!(cli_err.to_string().starts_with("serialization error:"));
}

#[test]
fn cli_result_ok_wraps_value() {
    let result: CliResult<i32> = Ok(42);
    assert_eq!(result.ok(), Some(42));
}

#[test]
fn cli_result_err_wraps_cli_error() {
    let result: CliResult<i32> = Err(CliError::Config("oops".to_string()));
    assert!(result.is_err());
}
