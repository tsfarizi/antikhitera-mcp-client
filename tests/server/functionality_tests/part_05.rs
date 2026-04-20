        #[test]
        fn $test_name() {
            // Gracefully skip if config files don't exist
            let config = match AppConfig::load(Some(Path::new("config/client.toml"))) {
                Ok(c) => c,
                Err(_) => {
                    eprintln!("SKIPPED: config/client.toml or config/model.toml not found");
                    return;
                }
            };

            let server = match config.servers.iter().find(|s| s.name == $server_name) {
                Some(s) => s,
                None => {
                    eprintln!("SKIPPED: Server '{}' not configured", $server_name);
                    return;
                }
            };

            // Check if this is a STDIO server with command path
            let cmd_path = match &server.command {
                Some(path) => path,
                None => {
                    eprintln!("SKIPPED: Server '{}' is not a STDIO server", $server_name);
                    return;
                }
            };

            if !cmd_path.exists() {
                eprintln!("SKIPPED: {} not found", cmd_path.display());
                return;
            }

            let mut client =
                McpTestClient::spawn(server).expect(&format!("Failed to spawn '{}'", $server_name));

            thread::sleep(Duration::from_millis(100));

            // Test initialize
            let init = client.initialize().expect("Initialize failed");
            assert!(init.get("result").is_some() || init.get("error").is_none());

            // Test list tools
            let tools = client.list_tools().expect("List tools failed");
            if let Some(result) = tools.get("result") {
                if let Some(t) = result.get("tools") {
                    println!(
                        "âœ“ '{}' provides {} tools",
                        $server_name,
                        t.as_array().map(|a| a.len()).unwrap_or(0)
                    );
                }
            }
        }
    };
}

// ============================================================================
// Generated Tests
// ============================================================================

server_test!(time_server_responds_to_initialize, "time");
server_test!(certificate_server_responds_to_initialize, "certificate");
