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
