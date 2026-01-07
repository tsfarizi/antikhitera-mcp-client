// Wizard tests - verifying configuration wizard functionality
//
// Tests for wizard configuration generation including HTTP server support.

mod http_server_generation_tests {
    use std::collections::HashMap;

    /// Helper to generate HTTP server TOML block (mirrors client::add_http_server logic)
    fn generate_http_server_toml(
        name: &str,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> String {
        let headers_toml = if headers.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = headers
                .iter()
                .map(|(k, v)| format!("{} = \"{}\"", k, v))
                .collect();
            format!("\nheaders = {{ {} }}", pairs.join(", "))
        };

        format!(
            r#"
[[servers]]
name = "{}"
url = "{}"{}"#,
            name, url, headers_toml
        )
    }

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

    #[test]
    fn test_http_server_toml_with_multiple_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("X-API-Key".to_string(), "key123".to_string());

        let result =
            generate_http_server_toml("multi-auth-api", "https://api.example.com", &headers);

        assert!(result.contains("headers = {"));
        // Both headers should be present
        assert!(result.contains("Authorization"));
        assert!(result.contains("X-API-Key"));
    }

    #[test]
    fn test_stdio_server_toml_format() {
        // Mirrors client::add_server logic
        let args: Vec<String> = vec!["--port".to_string(), "8080".to_string()];
        let args_toml = format!(
            "\nargs = [{}]",
            args.iter()
                .map(|a| format!("\"{}\"", a))
                .collect::<Vec<String>>()
                .join(", ")
        );

        let server_block = format!(
            r#"
[[servers]]
name = "{}"
command = "{}"{}"#,
            "local-server", "C:\\path\\to\\server.exe", args_toml
        );

        assert!(server_block.contains("[[servers]]"));
        assert!(server_block.contains(r#"name = "local-server""#));
        assert!(server_block.contains("command = "));
        assert!(server_block.contains(r#"args = ["--port", "8080"]"#));
    }
}

mod mask_sensitive_tests {
    /// Mask sensitive values for display (mirrors mod.rs logic)
    fn mask_sensitive(value: &str) -> String {
        if value.len() <= 8 {
            "*".repeat(value.len())
        } else {
            format!("{}...{}", &value[..4], &value[value.len() - 4..])
        }
    }

    #[test]
    fn test_mask_short_value() {
        assert_eq!(mask_sensitive("abc"), "***");
        assert_eq!(mask_sensitive("12345678"), "********");
    }

    #[test]
    fn test_mask_long_value() {
        let result = mask_sensitive("Bearer token12345");
        assert!(result.starts_with("Bear"));
        assert!(result.ends_with("2345"));
        assert!(result.contains("..."));
    }

    #[test]
    fn test_mask_empty_value() {
        assert_eq!(mask_sensitive(""), "");
    }

    #[test]
    fn test_mask_exact_boundary() {
        // 9 characters should trigger long format
        let result = mask_sensitive("123456789");
        assert_eq!(result, "1234...6789");
    }
}

mod transport_config_tests {
    use antikhitera_mcp_client::config::{ServerConfig, TransportType};
    use std::collections::HashMap;
    use std::path::PathBuf;

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
