    #[test]
    fn test_create_stdio_server_config() {
        let config = ServerConfig {
            name: "test-stdio".to_string(),
            transport: TransportType::Stdio,
            command: Some(PathBuf::from("/path/to/server")),
            args: vec!["--arg1".to_string()],
            env: HashMap::new(),
            workdir: None,
            url: None,
            headers: HashMap::new(),
            default_timezone: None,
            default_city: None,
        };

        assert!(config.is_stdio());
        assert!(config.command().is_some());
    }


    #[test]
    fn test_create_http_server_config() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let config = ServerConfig {
            name: "test-http".to_string(),
            transport: TransportType::Http,
            command: None,
            args: vec![],
            env: HashMap::new(),
            workdir: None,
            url: Some("https://api.example.com/mcp".to_string()),
            headers,
            default_timezone: None,
            default_city: None,
        };

        assert!(config.is_http());
        assert_eq!(config.url(), Some("https://api.example.com/mcp"));
        assert_eq!(config.headers.len(), 1);
    }
}
