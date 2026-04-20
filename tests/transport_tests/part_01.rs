    #[test]
    fn test_http_transport_config_creation() {
        let config = HttpTransportConfig {
            name: "test-server".to_string(),
            url: "https://mcp.example.com".to_string(),
            headers: HashMap::new(),
            mode: TransportMode::Auto,
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert_eq!(transport.get_name(), "test-server");
    }

