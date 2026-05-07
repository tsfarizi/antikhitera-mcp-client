//! Server Scanner
//!
//! This module provides functionality for scanning a directory to discover
//! MCP server binaries. It identifies executable files that can be loaded
//! as MCP servers.
//!
//! # Overview
//!
//! The scanner performs the following steps:
//! 1. Validate that the servers folder exists
//! 2. Iterate through all files in the folder
//! 3. Filter for executable files (platform-specific)
//! 4. Create `DiscoveredServer` entries for each valid binary
//!
//! # Example
//!
//! ```ignore
//! use antikythera_core::application::discovery::scanner;
//!
//! let servers = scanner::scan_folder("servers")?;
//! for server in servers {
//!     println!("Found: {}", server.name);
//! }
//! ```

use super::types::{DiscoveredServer, DiscoveryError};
use crate::logging::DiscoveryLogger;
use std::path::Path;

/// Scan a folder for MCP server binaries.
///
/// This function reads the contents of the specified folder and identifies
/// all executable files that could be MCP servers.
///
/// # Arguments
///
/// * `folder_path` - Path to the folder containing server binaries
///
/// # Returns
///
/// A vector of `DiscoveredServer` instances, one for each valid binary found.
/// Each server will have `LoadStatus::Pending` until it is actually loaded.
///
/// # Errors
///
/// Returns `DiscoveryError` if:
/// - The folder does not exist
/// - The folder cannot be read
/// - No executable files are found (optional, can return empty vec instead)
///
/// # Example
///
/// ```ignore
/// let servers = scan_folder("./servers")?;
/// println!("Found {} servers", servers.len());
/// ```
pub fn scan_folder(folder_path: impl AsRef<Path>) -> Result<Vec<DiscoveredServer>, DiscoveryError> {
    let log = DiscoveryLogger::new("discovery");
    let folder = folder_path.as_ref();

    log.info(format!(
        "Scanning servers folder | path={}",
        folder.display()
    ));

    // Check if folder exists
    if !folder.exists() {
        log.warn(format!(
            "Servers folder not found | path={}",
            folder.display()
        ));
        return Err(DiscoveryError::FolderNotFound {
            path: folder.to_path_buf(),
        });
    }

    if !folder.is_dir() {
        log.warn(format!(
            "Path is not a directory | path={}",
            folder.display()
        ));
        return Err(DiscoveryError::FolderNotFound {
            path: folder.to_path_buf(),
        });
    }

    // Read directory contents
    let entries = std::fs::read_dir(folder).map_err(|e| {
        log.warn(format!(
            "Failed to read servers folder | path={} error={}",
            folder.display(),
            e
        ));
        DiscoveryError::ReadError { source: e }
    })?;

    let mut servers = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log.warn(format!(
                    "Failed to read directory entry, skipping | error={}",
                    e
                ));
                continue;
            }
        };

        let path = entry.path();
        log.debug(format!("Checking file | path={}", path.display()));

        // Skip directories
        if path.is_dir() {
            log.debug(format!("Skipping directory | path={}", path.display()));
            continue;
        }

        // Check if file is executable
        if !is_executable(&path) {
            log.debug(format!(
                "Skipping non-executable file | path={}",
                path.display()
            ));
            continue;
        }

        // Extract server name from filename
        let name = extract_server_name(&path);
        log.debug(format!(
            "Found MCP server binary | name={} path={}",
            name,
            path.display()
        ));

        servers.push(DiscoveredServer::new(name, path));
    }

    log.info(format!(
        "Server scan complete | count={} path={}",
        servers.len(),
        folder.display()
    ));

    Ok(servers)
}

/// Extract a server name from a binary path.
///
/// The name is derived from the filename without extension.
/// For example: `/path/to/mcp-time.exe` -> `mcp-time`
///
/// # Arguments
///
/// * `path` - Path to the binary file
///
/// # Returns
///
/// The extracted server name as a String
pub fn extract_server_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Check if a file is executable.
///
/// This function uses platform-specific logic:
/// - **Windows**: Checks for `.exe`, `.cmd`, or `.bat` extensions
/// - **Unix**: Checks the executable permission bit
///
/// # Arguments
///
/// * `path` - Path to the file to check
///
/// # Returns
///
/// `true` if the file is considered executable, `false` otherwise
#[cfg(target_os = "windows")]
pub fn is_executable(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("exe") | Some("cmd") | Some("bat"))
}

#[cfg(all(unix, not(target_os = "windows")))]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = path.metadata() {
        let mode = metadata.permissions().mode();
        // Check if any execute bit is set (owner, group, or other)
        mode & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(any(target_os = "windows", unix)))]
pub fn is_executable(_path: &Path) -> bool {
    false
}
