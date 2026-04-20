    #[test]
    fn test_server_config_with_both_command_and_url() {
        use std::path::PathBuf;

        // When both command and url are provided, transport type determines behavior
        let config = ServerConfig {
            name: "hybrid".to_string(),
            transport: TransportType::Http,
            command: Some(PathBuf::from("/fallback/path")),
            args: vec![],
            env: HashMap::new(),
            workdir: None,
            url: Some("https://api.example.com".to_string()),
            headers: HashMap::new(),
            default_timezone: None,
            default_city: None,
        };

        // With HTTP transport, url should be used
        assert!(config.is_http());
        assert!(config.url().is_some());
    }


    #[test]
    fn test_server_config_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("X-API-Key".to_string(), "key456".to_string());

        let config = ServerConfig {
            name: "api-server".to_string(),
            transport: TransportType::Http,
            command: None,
            args: vec![],
            env: HashMap::new(),
            workdir: None,
            url: Some("https://api.example.com/mcp".to_string()),
            headers: headers.clone(),
            default_timezone: None,
            default_city: None,
        };

        assert_eq!(config.headers.len(), 2);
        assert_eq!(
            config.headers.get("Authorization"),
            Some(&"Bearer token123".to_string())
        );
        assert_eq!(config.headers.get("X-API-Key"), Some(&"key456".to_string()));
    }
}

mod http_transport_async_tests {
    use antikythera_core::tooling::transport::{
        HttpTransport, HttpTransportConfig, McpTransport, TransportMode,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_http_transport_is_disconnected_initially() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://nonexistent.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert!(!transport.is_connected().await);
    }

    #[tokio::test]
    async fn test_http_transport_disconnect() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://test.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        transport.disconnect().await;
        assert!(!transport.is_connected().await);
    }

    #[tokio::test]
    async fn test_http_transport_instructions_none_initially() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://test.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert!(transport.instructions().await.is_none());
    }

    #[tokio::test]
    async fn test_http_transport_tool_metadata_none() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://test.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert!(transport.tool_metadata("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_http_transport_list_tools_initially_empty() {
        let config = HttpTransportConfig {
            name: "test".to_string(),
            url: "https://test.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert!(transport.list_tools().await.is_empty());
    }
}
