//! Content types for parsing MCP tool outputs.
//!
//! This module provides types for handling file content and attachments
//! from MCP tool responses.

mod parser;
mod types;

pub use parser::{ParsedOutput, parse_step_output};
pub use types::{FileContent, FileMetadata};
