//! Transport configuration types.
//!
//! Contains configuration structs and enums for HTTP transport.

use std::collections::HashMap;

/// Transport mode for HTTP MCP servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// Stateful mode using SSE for session management
    Stateful,
    /// Stateless mode using direct HTTP POST (no SSE)
    Stateless,
    /// Auto-detect mode - tries SSE first, falls back to stateless
    Auto,
}

impl Default for TransportMode {
    fn default() -> Self {
        TransportMode::Auto
    }
}

/// HTTP Transport configuration.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Server name identifier
    pub name: String,
    /// Base URL for the MCP server
    pub url: String,
    /// Optional authorization headers
    pub headers: HashMap<String, String>,
    /// Transport mode (default: Auto)
    pub mode: TransportMode,
}
