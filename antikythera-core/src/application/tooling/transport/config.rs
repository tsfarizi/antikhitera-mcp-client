//! Transport configuration types.
//!
//! Contains configuration structs and enums for HTTP transport.

use std::collections::HashMap;

/// Declares capabilities offered by a transport implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportCapability {
    /// Bidirectional request/response over stateless HTTP.
    StatelessRpc,
    /// Stateful sessions (for example SSE + session endpoint).
    StatefulSession,
    /// Server-pushed events/notifications.
    StreamingEvents,
    /// Transport supports carrying caller metadata/correlation headers.
    MetadataPropagation,
}

/// Transport mode for HTTP MCP servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportMode {
    /// Stateful mode using SSE for session management
    Stateful,
    /// Stateless mode using direct HTTP POST (no SSE)
    Stateless,
    /// Auto-detect mode - tries SSE first, falls back to stateless
    #[default]
    Auto,
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
    /// Optional required capabilities for selection/negotiation.
    pub required_capabilities: Vec<TransportCapability>,
}

impl HttpTransportConfig {
    /// Returns true when all required capabilities are present.
    pub fn is_compatible_with(&self, offered: &[TransportCapability]) -> bool {
        self.required_capabilities
            .iter()
            .all(|required| offered.contains(required))
    }
}
