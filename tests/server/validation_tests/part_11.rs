#[test]
fn test_default_folder_constant() {
    assert_eq!(DEFAULT_SERVERS_FOLDER, "servers");
}

#[test]
fn test_re_exports_available() {
    let _ = DiscoveredServer::new("test", PathBuf::new());
    let _ = DiscoverySummary::default();
    let _ = LoadStatus::Pending;
}
