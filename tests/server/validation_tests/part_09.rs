#[test]
fn test_create_server_config() {
    let config =
        create_server_config("test-server", &PathBuf::from("/path/to/server"));

    assert_eq!(config.name, "test-server");
    assert_eq!(config.command, Some(PathBuf::from("/path/to/server")));
    assert!(config.args.is_empty());
    assert!(config.env.is_empty());
    assert!(config.workdir.is_none());
}

#[tokio::test]
async fn test_load_server_updates_status() {
    let mut server = DiscoveredServer::new(
        "nonexistent-server",
        PathBuf::from("/nonexistent/path"),
    );

    load_server(&mut server).await;

    // Should have failed status since the binary doesn't exist
    assert!(matches!(server.load_status, LoadStatus::Failed(_)));
}
