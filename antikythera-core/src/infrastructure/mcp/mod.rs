/// MCP protocol contracts and tools.
///
/// This module provides canonical contracts for MCP tool calling, including
/// envelope types for requests and responses, error mapping, and validation.
pub mod contract;

pub use contract::{ContractValidator, ToolCallEnvelope, ToolExecutionError, ToolResultEnvelope};
