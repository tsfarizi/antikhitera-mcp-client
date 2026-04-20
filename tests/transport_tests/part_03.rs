    #[test]
    fn test_server_config_stdio_transport() {
        use std::path::PathBuf;

        let config = ServerConfig {
            name: "test-stdio".to_string(),
            transport: TransportType::Stdio,
            command: Some(PathBuf::from("/path/to/server")),
            args: vec!["--port".to_string(), "8080".to_string()],
            env: HashMap::new(),
            workdir: None,
            url: None,
            headers: HashMap::new(),
            default_timezone: None,
            default_city: None,
        };

        assert!(config.is_stdio());
        assert!(!config.is_http());
        assert!(config.command().is_some());
        assert!(config.url().is_none());
    }


    #[test]
    fn test_server_config_http_transport() {
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

        assert!(!config.is_stdio());
        assert!(config.is_http());
        assert!(config.command().is_none());
        assert_eq!(config.url(), Some("https://api.example.com/mcp"));
    }

