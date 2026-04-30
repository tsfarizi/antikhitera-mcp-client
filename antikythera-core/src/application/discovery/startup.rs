//! Startup server discovery integration.
//!
//! Scans the `servers/` folder, attempts to load each discovered binary via
//! MCP stdio transport, retrieves their available tools, and returns a
//! structured [`StartupDiscoveryResult`].
//!
//! The result can be consumed by any embedding layer at startup to merge
//! newly found servers into an active configuration before the first client
//! session is opened, making discovered servers available without requiring
//! a config-file restart.  Helper methods on [`StartupDiscoveryResult`] —
//! [`loaded_servers`], [`failed_servers`], and [`has_loaded_servers`] — make
//! it easy to inspect and act on the discovery outcome.

use super::types::{DiscoveredServer, DiscoverySummary, LoadStatus};
use super::{DEFAULT_SERVERS_FOLDER, load_all, scan_folder};
use std::path::Path;
use crate::logging::DiscoveryLogger;

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
///   If None, uses DEFAULT_SERVERS_FOLDER ("servers")
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
/// use antikythera_core::application::discovery::startup;
///
/// let result = startup::run_startup_discovery(None).await;
/// if result.has_loaded_servers() {
///     println!("Ready with {} servers", result.summary.loaded);
/// }
/// ```
pub async fn run_startup_discovery(servers_folder: Option<&Path>) -> StartupDiscoveryResult {
    let log = DiscoveryLogger::new("discovery");
    let folder = servers_folder
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(DEFAULT_SERVERS_FOLDER).to_path_buf());

    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log.info("🔍 MCP Server Discovery");
    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log.info(format!("Scanning servers folder | path={}", folder.display()));

    // Check if folder exists
    if !folder.exists() {
        log.warn(format!("Servers folder not found - skipping discovery | path={}", folder.display()));
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
            log.error(format!("Failed to scan servers folder | error={}", e));
            return StartupDiscoveryResult {
                servers: Vec::new(),
                summary: DiscoverySummary::default(),
                folder_exists: true,
            };
        }
    };

    if servers.is_empty() {
        log.info("No server binaries found in folder");
        return StartupDiscoveryResult {
            servers: Vec::new(),
            summary: DiscoverySummary::default(),
            folder_exists: true,
        };
    }

    log.info(format!("Found server binaries | count={}", servers.len()));

    // Log each discovered binary
    for server in &servers {
        log.info(format!(
            "📦 Discovered server binary | name={} path={}",
            server.name,
            server.binary_path.display()
        ));
    }

    log.info("⏳ Loading servers and fetching tools via MCP...");

    // Load all servers
    let summary = load_all(&mut servers).await;

    // Log results for each server
    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log.info("📋 Discovery Results");
    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for server in &servers {
        match &server.load_status {
            LoadStatus::Success => {
                log.info(format!(
                    "✅ Server loaded successfully | name={} tools={}",
                    server.name,
                    server.tools.len()
                ));
                // Log each tool
                for (tool_name, description) in &server.tools {
                    let desc_preview: String = description.chars().take(50).collect();
                    log.info(format!(
                        "   🔧 Tool available | server={} tool={} desc={}",
                        server.name, tool_name, desc_preview
                    ));
                }
            }
            LoadStatus::NoTools => {
                log.warn(format!(
                    "⚠️  Server loaded but has no tools | name={}",
                    server.name
                ));
            }
            LoadStatus::Failed(err) => {
                log.error(format!(
                    "❌ Failed to load server | name={} error={}",
                    server.name, err
                ));
            }
            LoadStatus::Pending => {
                log.warn(format!("⏳ Server not loaded (pending) | name={}", server.name));
            }
        }
    }

    // Final summary
    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log.info(format!(
        "📊 Discovery Summary | total={} loaded={} failed={} no_tools={} total_tools={}",
        summary.total_found,
        summary.loaded,
        summary.failed,
        summary.no_tools,
        summary.total_tools
    ));
    log.info("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    StartupDiscoveryResult {
        servers,
        summary,
        folder_exists: true,
    }
}

/// Print discovery results to stdout (for non-logging scenarios).
///
/// Useful when tracing is disabled or when the embedding layer has not yet
/// configured a structured log subscriber.
pub fn print_discovery_summary(result: &StartupDiscoveryResult) {
    if !result.folder_exists {
        antikythera_log::cli_print!("⚠️  Servers folder not found - no auto-discovery performed");
        return;
    }

    if result.servers.is_empty() {
        antikythera_log::cli_print!("📂 No server binaries found in servers folder");
        return;
    }

    antikythera_log::cli_print!();
    antikythera_log::cli_print!("🔍 MCP Server Discovery Results:");
    antikythera_log::cli_print!("─────────────────────────────────");

    for server in &result.servers {
        match &server.load_status {
            LoadStatus::Success => {
                antikythera_log::cli_print!("  ✅ {} ({} tools)", server.name, server.tools.len());
                for (tool_name, _) in &server.tools {
                    antikythera_log::cli_print!("     └─ {}", tool_name);
                }
            }
            LoadStatus::NoTools => {
                antikythera_log::cli_print!("  ⚠️  {} (no tools)", server.name);
            }
            LoadStatus::Failed(e) => {
                antikythera_log::cli_print!("  ❌ {} - Error: {}", server.name, e);
            }
            LoadStatus::Pending => {}
        }
    }

    antikythera_log::cli_print!("─────────────────────────────────");
    antikythera_log::cli_print!(
        "📊 Total: {} servers | ✅ {} loaded | ❌ {} failed | 🔧 {} tools",
        result.summary.total_found,
        result.summary.loaded,
        result.summary.failed,
        result.summary.total_tools
    );
    antikythera_log::cli_print!();
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
