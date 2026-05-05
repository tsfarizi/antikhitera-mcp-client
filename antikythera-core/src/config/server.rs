//! # MCP Server Configuration
//!
//! This module defines configuration for connecting to MCP (Model Context Protocol) servers.
//! MCP servers provide tools that the AI agent can use to perform actions.
//!
//! ## Example - STDIO Server
//!
//! ```toml
//! [[servers]]
//! name = "time"
//! command = "python"
//! args = ["-m", "mcp_server_time"]
//! ```
//!
//! ## Example - HTTP Server
//!
//! ```toml
//! [[servers]]
//! name = "remote-api"
//! url = "https://mcp-server.example.com"
//! ```

use serde::{Deserialize, Serialize};
use shellexpand;
use std::collections::HashMap;
use std::path::PathBuf;

/// Transport type for MCP server connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    /// STDIO transport - spawns subprocess
    Stdio,
    /// HTTP transport - connects via HTTP/SSE
    Http,
    /// Builtin transport - runs in-process tool implementations
    Builtin,
}

/// Configuration for an MCP server connection.
///
/// MCP servers can be connected via STDIO (subprocess) or HTTP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Unique name for this server
    pub name: String,
    /// Transport type
    pub transport: TransportType,
    /// Path to the executable (for STDIO)
    pub command: Option<PathBuf>,
    /// Command line arguments (for STDIO)
    pub args: Vec<String>,
    /// Environment variables (for STDIO)
    pub env: HashMap<String, String>,
    /// Working directory (for STDIO)
    pub workdir: Option<PathBuf>,
    /// URL for HTTP transport
    pub url: Option<String>,
    /// HTTP headers (for HTTP transport)
    pub headers: HashMap<String, String>,
    /// Default timezone for time-related operations
    pub default_timezone: Option<String>,
    /// Default city for location-based operations
    pub default_city: Option<String>,
}

impl ServerConfig {
    /// Check if this is a STDIO transport server.
    pub fn is_stdio(&self) -> bool {
        matches!(self.transport, TransportType::Stdio)
    }

    /// Check if this is an HTTP transport server.
    pub fn is_http(&self) -> bool {
        matches!(self.transport, TransportType::Http)
    }

    /// Check if this is a builtin transport server.
    pub fn is_builtin(&self) -> bool {
        matches!(self.transport, TransportType::Builtin)
    }

    /// Get command path (for STDIO).
    pub fn command(&self) -> Option<&PathBuf> {
        self.command.as_ref()
    }

    /// Get URL (for HTTP).
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawServer {
    pub name: String,
    /// Command for STDIO transport (optional if url is provided)
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub workdir: Option<String>,
    /// URL for HTTP transport (optional if command is provided)
    pub url: Option<String>,
    /// HTTP headers for authentication
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub default_timezone: Option<String>,
    #[serde(default)]
    pub default_city: Option<String>,
}

impl From<RawServer> for ServerConfig {
    fn from(raw: RawServer) -> Self {
        let expand = |s: &str| -> String {
            shellexpand::full(s)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| s.to_string())
        };

        // Determine transport type based on provided fields
        let (transport, command, url) = if let Some(url_str) = raw.url {
            // HTTP transport
            (TransportType::Http, None, Some(url_str))
        } else if let Some(cmd_str) = raw.command {
            // STDIO transport
            let command_expanded = PathBuf::from(expand(&cmd_str));
            (TransportType::Stdio, Some(command_expanded), None)
        } else {
            // Default to Builtin (in-process handler)
            (TransportType::Builtin, None, None)
        };

        let workdir = raw.workdir.map(|d| PathBuf::from(expand(&d)));
        let args = raw.args.into_iter().map(|arg| expand(&arg)).collect();

        Self {
            name: raw.name,
            transport,
            command,
            args,
            env: raw.env,
            workdir,
            url,
            headers: raw.headers,
            default_timezone: raw.default_timezone,
            default_city: raw.default_city,
        }
    }
}

