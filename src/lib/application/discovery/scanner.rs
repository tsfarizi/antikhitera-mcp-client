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
//! use antikhitera_mcp_client::application::discovery::scanner;
//!
//! let servers = scanner::scan_folder("servers")?;
//! for server in servers {
//!     println!("Found: {}", server.name);
//! }
//! ```

use super::types::{DiscoveredServer, DiscoveryError};
use std::path::Path;
use tracing::{debug, info, trace, warn};

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
    let folder = folder_path.as_ref();

    info!(path = %folder.display(), "Scanning servers folder");

    // Check if folder exists
    if !folder.exists() {
        warn!(path = %folder.display(), "Servers folder not found");
        return Err(DiscoveryError::FolderNotFound {
            path: folder.to_path_buf(),
        });
    }

    if !folder.is_dir() {
        warn!(path = %folder.display(), "Path is not a directory");
        return Err(DiscoveryError::FolderNotFound {
            path: folder.to_path_buf(),
        });
    }

    // Read directory contents
    let entries = std::fs::read_dir(folder).map_err(|e| {
        warn!(path = %folder.display(), error = %e, "Failed to read servers folder");
        DiscoveryError::ReadError { source: e }
    })?;

    let mut servers = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "Failed to read directory entry, skipping");
                continue;
            }
        };

        let path = entry.path();
        trace!(path = %path.display(), "Checking file");

        // Skip directories
        if path.is_dir() {
            trace!(path = %path.display(), "Skipping directory");
            continue;
        }

        // Check if file is executable
        if !is_executable(&path) {
            trace!(path = %path.display(), "Skipping non-executable file");
            continue;
        }

        // Extract server name from filename
        let name = extract_server_name(&path);
        debug!(name = %name, path = %path.display(), "Found MCP server binary");

        servers.push(DiscoveredServer::new(name, path));
    }

    info!(
        count = servers.len(),
        path = %folder.display(),
        "Server scan complete"
    );

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
fn extract_server_name(path: &Path) -> String {
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
fn is_executable(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("exe") | Some("cmd") | Some("bat"))
}

#[cfg(not(target_os = "windows"))]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = path.metadata() {
        let mode = metadata.permissions().mode();
        // Check if any execute bit is set (owner, group, or other)
        mode & 0o111 != 0
    } else {
        false
    }
}

/// Check if a folder exists and is valid for scanning.
///
/// # Arguments
///
/// * `folder_path` - Path to check
///
/// # Returns
///
/// `true` if the folder exists and is a directory
pub fn folder_exists(folder_path: impl AsRef<Path>) -> bool {
    let path = folder_path.as_ref();
    path.exists() && path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_extract_server_name() {
        let path = Path::new("/path/to/mcp-time.exe");
        assert_eq!(extract_server_name(path), "mcp-time");

        let path = Path::new("server-name");
        assert_eq!(extract_server_name(path), "server-name");
    }

    #[test]
    fn test_folder_exists() {
        let dir = tempdir().unwrap();
        assert!(folder_exists(dir.path()));
        assert!(!folder_exists("/nonexistent/path"));
    }

    #[test]
    fn test_scan_empty_folder() {
        let dir = tempdir().unwrap();
        let result = scan_folder(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_scan_nonexistent_folder() {
        let result = scan_folder("/nonexistent/folder/path");
        assert!(matches!(result, Err(DiscoveryError::FolderNotFound { .. })));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_is_executable_windows() {
        assert!(is_executable(Path::new("server.exe")));
        assert!(is_executable(Path::new("server.EXE")));
        assert!(is_executable(Path::new("script.cmd")));
        assert!(is_executable(Path::new("script.bat")));
        assert!(!is_executable(Path::new("readme.txt")));
        assert!(!is_executable(Path::new("config.json")));
    }

    #[test]
    fn test_scan_folder_with_files() {
        let dir = tempdir().unwrap();

        // Create test files
        #[cfg(target_os = "windows")]
        {
            File::create(dir.path().join("server1.exe")).unwrap();
            File::create(dir.path().join("server2.exe")).unwrap();
            File::create(dir.path().join("readme.txt")).unwrap();
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.path().join("server1");
            File::create(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

            let path2 = dir.path().join("server2");
            File::create(&path2).unwrap();
            fs::set_permissions(&path2, fs::Permissions::from_mode(0o755)).unwrap();

            File::create(dir.path().join("readme.txt")).unwrap();
        }

        let servers = scan_folder(dir.path()).unwrap();

        // Should find 2 executables, not the readme.txt
        assert_eq!(servers.len(), 2);
    }
}
