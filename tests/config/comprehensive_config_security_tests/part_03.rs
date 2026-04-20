// ============================================================================
// INJECTION PREVENTION TESTS
// ============================================================================

#[test]
fn test_sql_injection_in_provider_id() {
    let config = ModelProviderConfig {
        id: "'; DROP TABLE providers; --".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    // Config layer stores as-is; caller is responsible for validation
    assert_eq!(config.id, "'; DROP TABLE providers; --");
}

#[test]
fn test_command_injection_in_server_args() {
    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("node")),
        args: vec!["'; rm -rf /; echo '".to_string()],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    // Config layer stores as-is; caller validates before execution
    assert_eq!(config.args[0], "'; rm -rf /; echo '");
}

#[test]
fn test_path_traversal_in_command() {
    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("../../../../etc/passwd")),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.command.as_ref().unwrap(), &PathBuf::from("../../../../etc/passwd"));
}

