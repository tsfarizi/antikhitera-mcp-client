use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type CliResult<T> = Result<T, CliError>;
