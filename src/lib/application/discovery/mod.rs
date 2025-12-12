//! MCP Server Discovery Module
//!
//! This module provides automatic discovery and loading of MCP server binaries
//! from a designated folder. It scans for executable files, spawns them as
//! MCP servers, and retrieves their available tools via the standard
//! `tools/list` method.
//!
//! # Architecture
//!
//! The discovery system consists of three main components:
//!
//! - **Scanner** (`scanner.rs`): Scans a folder for executable files
//! - **Loader** (`loader.rs`): Spawns servers and fetches tools via MCP
//! - **Types** (`types.rs`): Core data structures for discovery results
//!
//! # Usage
//!
//! ## Basic Scan and Load
//!
//! ```ignore
//! use antikhitera_mcp_client::application::discovery;
//!
//! // Scan and load in one step
//! let (servers, summary) = discovery::scan_and_load("servers").await?;
//!
//! println!("Found {} servers with {} tools",
//!     summary.loaded,
//!     summary.total_tools
//! );
//!
//! // Iterate through loaded servers
//! for server in &servers {
//!     if server.is_loaded() {
//!         println!("{}: {} tools", server.name, server.tool_count());
//!         for (tool_name, desc) in &server.tools {
//!             println!("  - {}: {}", tool_name, desc);
//!         }
//!     }
//! }
//! ```
//!
//! ## Manual Two-Step Process
//!
//! ```ignore
//! use antikhitera_mcp_client::application::discovery::{scanner, loader};
//!
//! // Step 1: Scan for servers
//! let mut servers = scanner::scan_folder("servers")?;
//! println!("Found {} potential servers", servers.len());
//!
//! // Step 2: Load each server
//! let summary = loader::load_all(&mut servers).await;
//!
//! for server in &servers {
//!     match &server.load_status {
//!         LoadStatus::Success => println!("✓ {} loaded", server.name),
//!         LoadStatus::Failed(e) => println!("✗ {} failed: {}", server.name, e),
//!         LoadStatus::NoTools => println!("⚠ {} has no tools", server.name),
//!         LoadStatus::Pending => unreachable!(),
//!     }
//! }
//! ```
//!
//! # Server Binary Requirements
//!
//! Server binaries must:
//! - Be executable files (platform-specific)
//! - Implement the MCP protocol via stdio
//! - Support `initialize` and `tools/list` methods
//!
//! On Windows, recognized extensions are: `.exe`, `.cmd`, `.bat`
//! On Unix, files must have the executable permission bit set.
//!
//! # Folder Structure
//!
//! The default servers folder structure:
//!
//! ```text
//! servers/
//! ├── mcp-time.exe        # Time-related tools
//! ├── mcp-filesystem.exe  # File system tools
//! ├── mcp-weather.exe     # Weather tools
//! └── ...
//! ```

pub mod loader;
pub mod scanner;
pub mod types;

// Re-export commonly used items
pub use loader::{load_all, load_server, scan_and_load};
pub use scanner::{folder_exists, scan_folder};
pub use types::{DiscoveredServer, DiscoveryError, DiscoverySummary, LoadStatus};

/// Default folder name for MCP server binaries.
pub const DEFAULT_SERVERS_FOLDER: &str = "servers";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_folder_constant() {
        assert_eq!(DEFAULT_SERVERS_FOLDER, "servers");
    }

    #[test]
    fn test_re_exports_available() {
        // Verify re-exports are accessible
        let _ = DiscoveredServer::new("test", std::path::PathBuf::new());
        let _ = DiscoverySummary::default();
        let _ = LoadStatus::Pending;
    }
}
