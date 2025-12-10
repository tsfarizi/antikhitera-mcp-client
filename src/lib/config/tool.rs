//! # Tool Configuration
//!
//! This module defines tool configuration synced from MCP servers.
//! Tools are capabilities provided by MCP servers that the AI agent can invoke.
//!
//! ## Example
//!
//! ```toml
//! [[tools]]
//! name = "get_current_time"
//! description = "Get the current time in a timezone"
//! server = "time"
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Configuration for an available tool.
///
/// Tools are synced from MCP servers and define what capabilities
/// are available to the AI agent.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ToolConfig {
    /// Unique name of the tool (e.g., "get_current_time")
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: Option<String>,
    /// Name of the MCP server that provides this tool
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawTool {
    Name(String),
    Detailed {
        name: String,
        description: Option<String>,
        #[serde(default)]
        server: Option<String>,
    },
}

impl From<RawTool> for ToolConfig {
    fn from(value: RawTool) -> Self {
        match value {
            RawTool::Name(name) => Self {
                name,
                description: None,
                server: None,
            },
            RawTool::Detailed {
                name,
                description,
                server,
            } => Self {
                name,
                description,
                server,
            },
        }
    }
}
