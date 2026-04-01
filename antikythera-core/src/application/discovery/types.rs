//! Discovery Types
//!
//! This module defines the core types used for MCP server discovery.
//! These types represent the results of scanning a servers folder
//! and loading server binaries to fetch their available tools.

use std::path::PathBuf;
use thiserror::Error;

/// Information about a discovered MCP server binary.
///
/// This struct holds metadata about a server binary found in the servers folder,
/// along with the tools it provides (if successfully loaded).
///
/// # Example
///
/// ```ignore
/// let server = DiscoveredServer {
///     name: "mcp-time".to_string(),
///     binary_path: PathBuf::from("servers/mcp-time.exe"),
///     tools: vec![
///         ("get_current_time".to_string(), "Get the current time".to_string())
///     ],
///     load_status: LoadStatus::Success,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    /// Server name derived from binary filename (without extension)
    pub name: String,
    /// Full path to the server binary
    pub binary_path: PathBuf,
    /// List of tools: (tool_name, description)
    pub tools: Vec<(String, String)>,
    /// Status of loading this server
    pub load_status: LoadStatus,
}

impl DiscoveredServer {
    /// Create a new discovered server with pending status.
    ///
    /// # Arguments
    ///
    /// * `name` - The server name (derived from binary filename)
    /// * `binary_path` - Full path to the binary file
    pub fn new(name: impl Into<String>, binary_path: PathBuf) -> Self {
        Self {
            name: name.into(),
            binary_path,
            tools: Vec::new(),
            load_status: LoadStatus::Pending,
        }
    }

    /// Check if the server was successfully loaded.
    pub fn is_loaded(&self) -> bool {
        matches!(self.load_status, LoadStatus::Success)
    }

    /// Get the number of tools provided by this server.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// Status of loading a server and fetching its tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    /// Server has not been loaded yet
    Pending,
    /// Server loaded successfully and tools retrieved
    Success,
    /// Server failed to load
    Failed(String),
    /// Server loaded but has no tools
    NoTools,
}

impl LoadStatus {
    /// Check if status indicates a successful load.
    pub fn is_success(&self) -> bool {
        matches!(self, LoadStatus::Success)
    }

    /// Get error message if failed.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            LoadStatus::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Summary of a discovery operation.
///
/// Contains aggregate statistics about the discovery and loading process.
#[derive(Debug, Clone, Default)]
pub struct DiscoverySummary {
    /// Total servers found in folder
    pub total_found: usize,
    /// Servers successfully loaded
    pub loaded: usize,
    /// Servers that failed to load
    pub failed: usize,
    /// Servers with no tools
    pub no_tools: usize,
    /// Total tools discovered across all servers
    pub total_tools: usize,
}

impl DiscoverySummary {
    /// Create a summary from a list of discovered servers.
    pub fn from_servers(servers: &[DiscoveredServer]) -> Self {
        let mut summary = Self::default();
        summary.total_found = servers.len();

        for server in servers {
            match &server.load_status {
                LoadStatus::Success => {
                    summary.loaded += 1;
                    summary.total_tools += server.tools.len();
                }
                LoadStatus::Failed(_) => summary.failed += 1,
                LoadStatus::NoTools => summary.no_tools += 1,
                LoadStatus::Pending => {}
            }
        }

        summary
    }
}

/// Errors that can occur during server discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The servers folder does not exist
    #[error("Servers folder not found: {path}")]
    FolderNotFound { path: PathBuf },

    /// Failed to read the servers folder
    #[error("Failed to read servers folder: {source}")]
    ReadError {
        #[source]
        source: std::io::Error,
    },

    /// Failed to load a specific server
    #[error("Failed to load server '{server}': {message}")]
    LoadError { server: String, message: String },

    /// No executable files found in folder
    #[error("No executable files found in servers folder")]
    NoExecutables,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_server_new() {
        let server = DiscoveredServer::new("test-server", PathBuf::from("/path/to/server"));
        assert_eq!(server.name, "test-server");
        assert_eq!(server.binary_path, PathBuf::from("/path/to/server"));
        assert!(server.tools.is_empty());
        assert_eq!(server.load_status, LoadStatus::Pending);
    }

    #[test]
    fn test_discovered_server_is_loaded() {
        let mut server = DiscoveredServer::new("test", PathBuf::from("/test"));
        assert!(!server.is_loaded());

        server.load_status = LoadStatus::Success;
        assert!(server.is_loaded());
    }

    #[test]
    fn test_load_status_error_message() {
        let pending = LoadStatus::Pending;
        assert!(pending.error_message().is_none());

        let failed = LoadStatus::Failed("connection error".to_string());
        assert_eq!(failed.error_message(), Some("connection error"));
    }

    #[test]
    fn test_discovery_summary_from_servers() {
        let servers = vec![
            {
                let mut s = DiscoveredServer::new("s1", PathBuf::from("/s1"));
                s.load_status = LoadStatus::Success;
                s.tools = vec![("t1".to_string(), "desc".to_string())];
                s
            },
            {
                let mut s = DiscoveredServer::new("s2", PathBuf::from("/s2"));
                s.load_status = LoadStatus::Failed("error".to_string());
                s
            },
            {
                let mut s = DiscoveredServer::new("s3", PathBuf::from("/s3"));
                s.load_status = LoadStatus::NoTools;
                s
            },
        ];

        let summary = DiscoverySummary::from_servers(&servers);
        assert_eq!(summary.total_found, 3);
        assert_eq!(summary.loaded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.no_tools, 1);
        assert_eq!(summary.total_tools, 1);
    }
}
