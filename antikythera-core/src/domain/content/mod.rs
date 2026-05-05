//! Content types for parsing MCP tool outputs.
//!
//! This module provides types for handling file content and attachments
//! from MCP tool responses.

pub mod parser;
pub mod types;

pub use parser::{ParsedOutput, parse_step_output};
pub use types::{ContentItem, FileContent, FileMetadata};
