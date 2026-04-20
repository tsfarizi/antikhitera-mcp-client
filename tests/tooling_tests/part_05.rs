#[tokio::test]
async fn test_spawn_and_list_tools_stdio_invalid_command() {
    let config = ServerConfig {
        name: "test_stdio".to_string(),
        transport: TransportType::Stdio,
        command: Some("non_existent_command_xyz".into()),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let result = spawn_and_list_tools(&config).await;
    assert!(result.is_err());
    // The error should be related to spawning the process or configuration
}
