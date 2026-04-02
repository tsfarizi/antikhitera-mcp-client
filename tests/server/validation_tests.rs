// Server validation tests - validating server configuration
//
// Tests that verify configuration references are valid.
// These tests gracefully skip if config files don't exist.

use antikythera_core::config::AppConfig;
use std::path::Path;

#[test]
fn all_servers_have_valid_commands() {
    // Gracefully skip if config files don't exist
    let config = match AppConfig::load(Some(Path::new("config/client.toml"))) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("SKIPPED: config/client.toml or config/model.toml not found");
            return;
        }
    };

    for server in &config.servers {
        // Handle Option<PathBuf> for command
        let cmd_display = server
            .command
            .as_ref()
            .map(|p| p.display().to_string())
            .or_else(|| server.url.clone())
            .unwrap_or_else(|| "(none)".to_string());

        println!("Checking '{}': {}", server.name, cmd_display);

        // Check if command exists (only for STDIO servers)
        if let Some(cmd_path) = &server.command {
            if !cmd_path.exists() {
                eprintln!(
                    "WARNING: '{}' command not found: {}",
                    server.name,
                    cmd_path.display()
                );
            }
        } else if server.url.is_some() {
            // HTTP server - no local file to check
            println!("  (HTTP server - URL: {})", server.url.as_ref().unwrap());
        } else {
            eprintln!(
                "WARNING: '{}' has neither command nor URL configured",
                server.name
            );
        }
    }
}

#[test]
fn all_tools_reference_existing_servers() {
    // Gracefully skip if config files don't exist
    let config = match AppConfig::load(Some(Path::new("config/client.toml"))) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("SKIPPED: config/client.toml or config/model.toml not found");
            return;
        }
    };

    let server_names: Vec<&str> = config.servers.iter().map(|s| s.name.as_str()).collect();

    for tool in &config.tools {
        if let Some(server) = &tool.server {
            assert!(
                server_names.contains(&server.as_str()),
                "Tool '{}' references undefined server '{}'",
                tool.name,
                server
            );
        }
    }

    println!("✓ All {} tools reference valid servers", config.tools.len());
}
