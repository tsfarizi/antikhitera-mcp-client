//! Startup Discovery Integration
//!
//! This module provides startup logging for MCP server discovery.
//! It scans the servers folder, loads each server, and logs the results.

use super::types::{DiscoveredServer, DiscoverySummary, LoadStatus};
use super::{DEFAULT_SERVERS_FOLDER, load_all, scan_folder};
use std::path::Path;
use tracing::{error, info, warn};

/// Result of the startup discovery process.
#[derive(Debug, Clone)]
pub struct StartupDiscoveryResult {
    /// All discovered servers
    pub servers: Vec<DiscoveredServer>,
    /// Summary statistics
    pub summary: DiscoverySummary,
    /// Whether the servers folder exists
    pub folder_exists: bool,
}

impl StartupDiscoveryResult {
    /// Check if any servers were successfully loaded.
    pub fn has_loaded_servers(&self) -> bool {
        self.summary.loaded > 0
    }

    /// Get all successfully loaded servers.
    pub fn loaded_servers(&self) -> Vec<&DiscoveredServer> {
        self.servers.iter().filter(|s| s.is_loaded()).collect()
    }

    /// Get all failed servers.
    pub fn failed_servers(&self) -> Vec<&DiscoveredServer> {
        self.servers
            .iter()
            .filter(|s| matches!(s.load_status, LoadStatus::Failed(_)))
            .collect()
    }
}

/// Run server discovery at startup and log results.
///
/// This function is designed to be called during application startup.
/// It scans the servers folder, attempts to load each server,
/// and logs comprehensive information about the results.
///
/// # Arguments
///
/// * `servers_folder` - Optional custom path to the servers folder.
///                      If None, uses DEFAULT_SERVERS_FOLDER ("servers")
///
/// # Returns
///
/// A `StartupDiscoveryResult` containing all discovered servers and statistics.
///
/// # Logging
///
/// The function logs:
/// - Starting discovery message
/// - Each server found during scanning
/// - Loading progress for each server
/// - Success/failure status with tool counts
/// - Final summary with statistics
///
/// # Example
///
/// ```ignore
/// use antikhitera_mcp_client::application::discovery::startup;
///
/// let result = startup::run_startup_discovery(None).await;
/// if result.has_loaded_servers() {
///     println!("Ready with {} servers", result.summary.loaded);
/// }
/// ```
pub async fn run_startup_discovery(servers_folder: Option<&Path>) -> StartupDiscoveryResult {
    let folder = servers_folder
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(DEFAULT_SERVERS_FOLDER).to_path_buf());

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🔍 MCP Server Discovery");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!(path = %folder.display(), "Scanning servers folder");

    // Check if folder exists
    if !folder.exists() {
        warn!(path = %folder.display(), "Servers folder not found - skipping discovery");
        return StartupDiscoveryResult {
            servers: Vec::new(),
            summary: DiscoverySummary::default(),
            folder_exists: false,
        };
    }

    // Scan for servers
    let mut servers = match scan_folder(&folder) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to scan servers folder");
            return StartupDiscoveryResult {
                servers: Vec::new(),
                summary: DiscoverySummary::default(),
                folder_exists: true,
            };
        }
    };

    if servers.is_empty() {
        info!("No server binaries found in folder");
        return StartupDiscoveryResult {
            servers: Vec::new(),
            summary: DiscoverySummary::default(),
            folder_exists: true,
        };
    }

    info!(count = servers.len(), "Found server binaries");

    // Log each discovered binary
    for server in &servers {
        info!(
            name = %server.name,
            path = %server.binary_path.display(),
            "📦 Discovered server binary"
        );
    }

    info!("⏳ Loading servers and fetching tools via MCP...");

    // Load all servers
    let summary = load_all(&mut servers).await;

    // Log results for each server
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📋 Discovery Results");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for server in &servers {
        match &server.load_status {
            LoadStatus::Success => {
                info!(
                    name = %server.name,
                    tools = server.tools.len(),
                    "✅ Server loaded successfully"
                );
                // Log each tool
                for (tool_name, description) in &server.tools {
                    let desc_preview: String = description.chars().take(50).collect();
                    info!(
                        server = %server.name,
                        tool = %tool_name,
                        desc = %desc_preview,
                        "   🔧 Tool available"
                    );
                }
            }
            LoadStatus::NoTools => {
                warn!(
                    name = %server.name,
                    "⚠️  Server loaded but has no tools"
                );
            }
            LoadStatus::Failed(err) => {
                error!(
                    name = %server.name,
                    error = %err,
                    "❌ Failed to load server"
                );
            }
            LoadStatus::Pending => {
                warn!(name = %server.name, "⏳ Server not loaded (pending)");
            }
        }
    }

    // Final summary
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!(
        total = summary.total_found,
        loaded = summary.loaded,
        failed = summary.failed,
        no_tools = summary.no_tools,
        total_tools = summary.total_tools,
        "📊 Discovery Summary"
    );
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    StartupDiscoveryResult {
        servers,
        summary,
        folder_exists: true,
    }
}

/// Print discovery results to stdout (for non-logging scenarios).
///
/// Useful when tracing is disabled or for CLI output.
pub fn print_discovery_summary(result: &StartupDiscoveryResult) {
    if !result.folder_exists {
        println!("⚠️  Servers folder not found - no auto-discovery performed");
        return;
    }

    if result.servers.is_empty() {
        println!("📂 No server binaries found in servers folder");
        return;
    }

    println!();
    println!("🔍 MCP Server Discovery Results:");
    println!("─────────────────────────────────");

    for server in &result.servers {
        match &server.load_status {
            LoadStatus::Success => {
                println!("  ✅ {} ({} tools)", server.name, server.tools.len());
                for (tool_name, _) in &server.tools {
                    println!("     └─ {}", tool_name);
                }
            }
            LoadStatus::NoTools => {
                println!("  ⚠️  {} (no tools)", server.name);
            }
            LoadStatus::Failed(e) => {
                println!("  ❌ {} - Error: {}", server.name, e);
            }
            LoadStatus::Pending => {}
        }
    }

    println!("─────────────────────────────────");
    println!(
        "📊 Total: {} servers | ✅ {} loaded | ❌ {} failed | 🔧 {} tools",
        result.summary.total_found,
        result.summary.loaded,
        result.summary.failed,
        result.summary.total_tools
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_discovery_result_methods() {
        let result = StartupDiscoveryResult {
            servers: vec![],
            summary: DiscoverySummary::default(),
            folder_exists: true,
        };
        assert!(!result.has_loaded_servers());
        assert!(result.loaded_servers().is_empty());
        assert!(result.failed_servers().is_empty());
    }
}
