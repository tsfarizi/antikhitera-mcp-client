//! Assembler errors.

use thiserror::Error;

/// Errors during UI component assembly.
#[derive(Debug, Error)]
pub enum AssemblerError {
    /// Data index out of bounds
    #[error("Data index {0} out of bounds (available: {1} steps)")]
    IndexOutOfBounds(usize, usize),

    /// Unknown component type
    #[error("Unknown component type: '{0}' not defined in ui.toml")]
    UnknownComponent(String),

    /// Missing required field
    #[error("Missing required field '{0}' in tool output")]
    MissingField(String),

    /// Type mismatch
    #[error("Type error: field '{field}' expected {expected}, got {actual}")]
    TypeError {
        field: String,
        expected: String,
        actual: String,
    },

    /// Invalid tool output structure
    #[error("Invalid tool output structure: {0}")]
    InvalidStructure(String),
}
