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
