#[test]
fn test_stdio_server_config() {
    let raw = RawServer {
        name: "test".to_string(),
        command: Some("/path/to/server".to_string()),
        args: vec!["--port".to_string(), "8080".to_string()],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let config = ServerConfig::from(raw);
    assert_eq!(config.name, "test");
    assert!(config.is_stdio());
    assert!(!config.is_http());
    assert!(config.command().is_some());
    assert!(config.url().is_none());
}

#[test]
fn test_http_server_config() {
    let raw = RawServer {
        name: "remote".to_string(),
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: Some("https://example.com/mcp".to_string()),
        headers: HashMap::from([(
            "Authorization".to_string(),
            "Bearer token".to_string(),
        )]),
        default_timezone: None,
        default_city: None,
    };

    let config = ServerConfig::from(raw);
    assert_eq!(config.name, "remote");
    assert!(!config.is_stdio());
    assert!(config.is_http());
    assert!(!config.is_builtin());
    assert!(config.command().is_none());
    assert_eq!(config.url(), Some("https://example.com/mcp"));
    assert_eq!(
        config.headers.get("Authorization"),
        Some(&"Bearer token".to_string())
    );
}

#[test]
fn test_builtin_server_config() {
    let raw = RawServer {
        name: "builtin_time".to_string(),
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let config = ServerConfig::from(raw);
    assert_eq!(config.name, "builtin_time");
    assert!(!config.is_stdio());
    assert!(!config.is_http());
    assert!(config.is_builtin());
    assert!(config.command().is_none());
    assert!(config.url().is_none());
}
