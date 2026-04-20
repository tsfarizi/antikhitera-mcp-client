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

