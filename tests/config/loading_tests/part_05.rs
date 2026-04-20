#[test]
fn loads_actual_config_files() {
    let config_dir = Path::new("config");
    let client_path = config_dir.join("client.toml");

    // Skip if config directory doesn't exist (e.g., in CI without config)
    if !client_path.exists() {
        eprintln!("Skipping: config/client.toml not found");
        return;
    }

    let config = AppConfig::load(Some(&client_path)).expect("Failed to load actual config files");

    // Basic validation that config loaded successfully
    assert!(!config.model.is_empty(), "model should not be empty");
    assert!(
        !config.default_provider.is_empty(),
        "default_provider should not be empty"
    );
    assert!(
        !config.providers.is_empty(),
        "providers should not be empty"
    );
}

/// Test that prompts section in actual model.toml loads correctly

#[test]
fn loads_actual_prompts_config() {
    let config_dir = Path::new("config");
    let client_path = config_dir.join("client.toml");

    // Skip if config directory doesn't exist
    if !client_path.exists() {
        eprintln!("Skipping: config/client.toml not found");
        return;
    }

    let config = AppConfig::load(Some(&client_path)).expect("Failed to load actual config files");

    // Verify prompt_template is loaded (should not be default if [prompts].template exists)
    let template = config.prompt_template();
    assert!(!template.is_empty(), "prompt template should not be empty");

    // If using Indonesian config, it should contain specific text
    if template.contains("Cakung") {
        assert!(
            template.contains("{{tool_guidance}}"),
            "template should have tool_guidance placeholder"
        );
    }
}

/// Test that tools from actual config are loaded

#[test]
fn loads_actual_tools_config() {
    let config_dir = Path::new("config");
    let client_path = config_dir.join("client.toml");

    // Skip if config directory doesn't exist
    if !client_path.exists() {
        eprintln!("Skipping: config/client.toml not found");
        return;
    }

    let config = AppConfig::load(Some(&client_path)).expect("Failed to load actual config files");

    // Verify tools are loaded if they exist in the config
    for tool in &config.tools {
        assert!(!tool.name.is_empty(), "tool name should not be empty");
    }
}
