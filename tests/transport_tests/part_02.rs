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
            required_capabilities: Vec::new(),
        };

        let transport = HttpTransport::new(config);
        assert_eq!(transport.get_name(), "auth-server");
    }

