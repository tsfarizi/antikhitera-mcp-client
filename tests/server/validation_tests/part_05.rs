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

    println!("âœ“ All {} tools reference valid servers", config.tools.len());
}
