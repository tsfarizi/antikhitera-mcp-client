//! # MCP Server Configuration
//!
//! This module defines configuration for connecting to MCP (Model Context Protocol) servers.
//! MCP servers provide tools that the AI agent can use to perform actions.
//!
//! ## Example
//!
//! ```toml
//! [[servers]]
//! name = "time"
//! command = "python"
//! args = ["-m", "mcp_server_time"]
//! ```

use serde::Deserialize;
use shellexpand;
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for an MCP server connection.
///
/// MCP servers are external processes that provide tools for the AI agent.
/// Each server is started as a subprocess with the specified command and arguments.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    /// Unique name for this server
    pub name: String,
    /// Path to the executable
    pub command: PathBuf,
    /// Command line arguments
    pub args: Vec<String>,
    /// Environment variables for the process
    pub env: HashMap<String, String>,
    /// Working directory (optional)
    pub workdir: Option<PathBuf>,
    /// Default timezone for time-related operations
    pub default_timezone: Option<String>,
    /// Default city for location-based operations
    pub default_city: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawServer {
    name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    workdir: Option<String>,
    #[serde(default)]
    default_timezone: Option<String>,
    #[serde(default)]
    default_city: Option<String>,
}

impl From<RawServer> for ServerConfig {
    fn from(raw: RawServer) -> Self {
        let expand = |s: &str| -> String {
            shellexpand::full(s)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| s.to_string())
        };

        let command_str = expand(&raw.command);
        let command = PathBuf::from(command_str);

        let workdir = raw.workdir.map(|d| PathBuf::from(expand(&d)));

        let args = raw.args.into_iter().map(|arg| expand(&arg)).collect();

        Self {
            name: raw.name,
            command,
            args,
            env: raw.env,
            workdir,
            default_timezone: raw.default_timezone,
            default_city: raw.default_city,
        }
    }
}
