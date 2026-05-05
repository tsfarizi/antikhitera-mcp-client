#[test]
fn test_extract_server_name() {
    let path = Path::new("/path/to/mcp-time.exe");
    assert_eq!(extract_server_name(path), "mcp-time");

    let path = Path::new("server-name");
    assert_eq!(extract_server_name(path), "server-name");
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

    #[cfg(all(unix, not(target_os = "windows")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.path().join("server1");
        File::create(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let path2 = dir.path().join("server2");
        File::create(&path2).unwrap();
        std::fs::set_permissions(&path2, std::fs::Permissions::from_mode(0o755)).unwrap();

        File::create(dir.path().join("readme.txt")).unwrap();
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        File::create(dir.path().join("server1")).unwrap();
        File::create(dir.path().join("server2")).unwrap();
        File::create(dir.path().join("readme.txt")).unwrap();
    }

    let servers = scan_folder(dir.path()).unwrap();

    // Should find 2 executables, not the readme.txt
    assert_eq!(servers.len(), 2);
}
