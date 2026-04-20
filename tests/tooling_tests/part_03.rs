#[tokio::test]
async fn test_spawn_and_list_tools_http_missing_url() {
    let config = ServerConfig {
        name: "test_http".to_string(),
        transport: TransportType::Http,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None, // Missing URL
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let result = spawn_and_list_tools(&config).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("missing URL for HTTP transport"));
}

