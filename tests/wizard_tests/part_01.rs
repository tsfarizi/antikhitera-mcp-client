    #[test]
    fn test_http_server_toml_without_headers() {
        let result =
            generate_http_server_toml("remote-api", "https://api.example.com/mcp", &HashMap::new());

        assert!(result.contains("[[servers]]"));
        assert!(result.contains(r#"name = "remote-api""#));
        assert!(result.contains(r#"url = "https://api.example.com/mcp""#));
        assert!(!result.contains("headers"));
    }


    #[test]
    fn test_http_server_toml_with_single_header() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let result =
            generate_http_server_toml("secure-api", "https://secure.example.com", &headers);

        assert!(result.contains("[[servers]]"));
        assert!(result.contains(r#"name = "secure-api""#));
        assert!(result.contains(r#"url = "https://secure.example.com""#));
        assert!(result.contains("headers = {"));
        assert!(result.contains(r#"Authorization = "Bearer token123""#));
    }

