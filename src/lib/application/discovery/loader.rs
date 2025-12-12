//! Server Loader
//!
//! This module handles loading MCP servers and fetching their available tools
//! via the standard MCP `tools/list` method.
//!
//! # Overview
//!
//! The loader performs the following for each discovered server:
//! 1. Creates a `ServerConfig` from the binary path
//! 2. Spawns the server process
//! 3. Sends MCP `initialize` request
//! 4. Sends `tools/list` request to get available tools
//! 5. Stores the result in the `DiscoveredServer` struct
//!
//! # Example
//!
//! ```ignore
//! use antikhitera_mcp_client::application::discovery::{scanner, loader};
//!
//! // First scan for servers
//! let mut servers = scanner::scan_folder("servers")?;
//!
//! // Then load each server and fetch tools
//! loader::load_all(&mut servers).await;
//!
//! for server in &servers {
//!     if server.is_loaded() {
//!         println!("{} has {} tools", server.name, server.tool_count());
//!     }
//! }
//! ```

use super::types::{DiscoveredServer, DiscoverySummary, LoadStatus};
use crate::config::ServerConfig;
use crate::tooling::spawn_and_list_tools;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Load all discovered servers and fetch their tools.
///
/// This function iterates through all discovered servers and attempts to
/// spawn each one, sending a `tools/list` request to retrieve available tools.
///
/// # Arguments
///
/// * `servers` - Mutable slice of discovered servers to load
///
/// # Returns
///
/// A `DiscoverySummary` containing statistics about the loading process.
///
/// # Note
///
/// This function modifies each server in place, updating their `tools` and
/// `load_status` fields based on the result of the loading attempt.
///
/// # Example
///
/// ```ignore
/// let mut servers = scanner::scan_folder("servers")?;
/// let summary = loader::load_all(&mut servers).await;
/// println!("Loaded {} servers with {} total tools", summary.loaded, summary.total_tools);
/// ```
pub async fn load_all(servers: &mut [DiscoveredServer]) -> DiscoverySummary {
    info!(count = servers.len(), "Loading discovered servers");

    for server in servers.iter_mut() {
        load_server(server).await;
    }

    let summary = DiscoverySummary::from_servers(servers);

    info!(
        loaded = summary.loaded,
        failed = summary.failed,
        total_tools = summary.total_tools,
        "Server loading complete"
    );

    summary
}

/// Load a single server and fetch its tools.
///
/// This function spawns the server process, performs MCP initialization,
/// and retrieves the list of available tools via `tools/list`.
///
/// # Arguments
///
/// * `server` - Mutable reference to the server to load
///
/// # Note
///
/// The function modifies the server in place, updating:
/// - `tools`: Vector of (name, description) tuples
/// - `load_status`: Result of the loading attempt
///
/// # Example
///
/// ```ignore
/// let mut server = DiscoveredServer::new("mcp-time", PathBuf::from("servers/mcp-time.exe"));
/// loader::load_server(&mut server).await;
/// if server.is_loaded() {
///     println!("Loaded {} tools", server.tool_count());
/// }
/// ```
pub async fn load_server(server: &mut DiscoveredServer) {
    debug!(
        name = %server.name,
        path = %server.binary_path.display(),
        "Loading server"
    );

    // Create ServerConfig from binary path
    let config = create_server_config(&server.name, &server.binary_path);

    // Spawn server and fetch tools
    match spawn_and_list_tools(&config).await {
        Ok(tools) => {
            if tools.is_empty() {
                info!(name = %server.name, "Server loaded but has no tools");
                server.load_status = LoadStatus::NoTools;
            } else {
                info!(
                    name = %server.name,
                    tool_count = tools.len(),
                    "Server loaded successfully"
                );

                // Log each tool for debugging
                for (name, desc) in &tools {
                    debug!(
                        server = %server.name,
                        tool = %name,
                        description = %desc,
                        "Discovered tool"
                    );
                }

                server.tools = tools;
                server.load_status = LoadStatus::Success;
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            error!(
                name = %server.name,
                error = %error_msg,
                "Failed to load server"
            );
            server.load_status = LoadStatus::Failed(error_msg);
        }
    }
}

/// Create a `ServerConfig` from a binary path.
///
/// This function creates the configuration needed to spawn an MCP server
/// process from just a binary path.
///
/// # Arguments
///
/// * `name` - Server name
/// * `binary_path` - Path to the server binary
///
/// # Returns
///
/// A `ServerConfig` ready for use with `spawn_and_list_tools`
fn create_server_config(name: &str, binary_path: &PathBuf) -> ServerConfig {
    ServerConfig {
        name: name.to_string(),
        command: binary_path.clone(),
        args: Vec::new(),
        env: HashMap::new(),
        workdir: None,
        default_timezone: None,
        default_city: None,
    }
}

/// Scan and load all servers from a folder in one operation.
///
/// This is a convenience function that combines scanning and loading.
///
/// # Arguments
///
/// * `folder_path` - Path to the servers folder
///
/// # Returns
///
/// A tuple of (Vec<DiscoveredServer>, DiscoverySummary)
///
/// # Errors
///
/// Returns `DiscoveryError` if scanning fails.
///
/// # Example
///
/// ```ignore
/// let (servers, summary) = loader::scan_and_load("servers").await?;
/// println!("Discovered {} servers", servers.len());
/// ```
pub async fn scan_and_load(
    folder_path: impl AsRef<std::path::Path>,
) -> Result<(Vec<DiscoveredServer>, DiscoverySummary), super::types::DiscoveryError> {
    use super::scanner;

    let mut servers = scanner::scan_folder(folder_path)?;

    if servers.is_empty() {
        warn!("No servers found in folder");
        return Ok((servers, DiscoverySummary::default()));
    }

    let summary = load_all(&mut servers).await;
    Ok((servers, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server_config() {
        let config = create_server_config("test-server", &PathBuf::from("/path/to/server"));

        assert_eq!(config.name, "test-server");
        assert_eq!(config.command, PathBuf::from("/path/to/server"));
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.workdir.is_none());
    }

    #[tokio::test]
    async fn test_load_server_updates_status() {
        // Test that load_server properly updates the status
        // Note: This will fail since there's no actual server, but it tests the error handling
        let mut server =
            DiscoveredServer::new("nonexistent-server", PathBuf::from("/nonexistent/path"));

        load_server(&mut server).await;

        // Should have failed status since the binary doesn't exist
        assert!(matches!(server.load_status, LoadStatus::Failed(_)));
    }
}
