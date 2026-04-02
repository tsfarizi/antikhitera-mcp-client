// Transport tests - verifying HTTP transport and MCP transport abstraction
//
// Tests for HTTP transport configuration and JSON-RPC over HTTP.

mod http_transport_tests {
    use antikythera_core::config::{ServerConfig, TransportType};
    use antikythera_core::tooling::transport::{
        HttpTransport, HttpTransportConfig, TransportMode,
    };
    use std::collections::HashMap;

    #[test]
    fn test_http_transport_config_creation() {
        let config = HttpTransportConfig {
            name: "test-server".to_string(),
            url: "https://mcp.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
        };

        let transport = HttpTransport::new(config);
        assert_eq!(transport.get_name(), "test-server");
    }

    #[test]
    fn test_http_transport_with_auth_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let config = HttpTransportConfig {
            name: "auth-server".to_string(),
            url: "https://secure.mcp.example.com/api".to_string(),
            headers,
            mode: TransportMode::Auto,
        };

        let transport = HttpTransport::new(config);
        assert_eq!(transport.get_name(), "auth-server");
    }

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

    #[test]
    fn test_transport_type_equality() {
        assert_eq!(TransportType::Stdio, TransportType::Stdio);
        assert_eq!(TransportType::Http, TransportType::Http);
        assert_ne!(TransportType::Stdio, TransportType::Http);
    }

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
        };

        let transport = HttpTransport::new(config);
        assert!(transport.list_tools().await.is_empty());
    }
}
