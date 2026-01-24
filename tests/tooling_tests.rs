// Tooling tests - verifying high-level tooling functions
//
// Tests for spawn_and_list_tools with different transport types.

use antikhitera_mcp_client::application::tooling::spawn_and_list_tools;
use antikhitera_mcp_client::config::{ServerConfig, TransportType};
use std::collections::HashMap;

#[tokio::test]
async fn test_spawn_and_list_tools_http_missing_url() {
    let config = ServerConfig {
        name: "test_http".to_string(),
        transport: TransportType::Http,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None, // Missing URL
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let result = spawn_and_list_tools(&config).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("missing URL for HTTP transport"));
}

#[tokio::test]
async fn test_spawn_and_list_tools_stdio_invalid_command() {
    let config = ServerConfig {
        name: "test_stdio".to_string(),
        transport: TransportType::Stdio,
        command: Some("non_existent_command_xyz".into()),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    let result = spawn_and_list_tools(&config).await;
    assert!(result.is_err());
    // The error should be related to spawning the process or configuration
}
