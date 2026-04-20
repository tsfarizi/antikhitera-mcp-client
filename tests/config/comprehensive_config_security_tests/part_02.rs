// ============================================================================
// SERVER CONFIG TESTS
// ============================================================================

#[test]
fn test_server_config_stdio() {
    let config = ServerConfig {
        name: "test-server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("/usr/bin/server")),
        args: vec!["--verbose".to_string()],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: Some("UTC".to_string()),
        default_city: None,
    };

    assert!(config.is_stdio());
    assert!(!config.is_http());
}

#[test]
fn test_server_config_http() {
    let config = ServerConfig {
        name: "remote-server".to_string(),
        transport: TransportType::Http,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: Some("http://localhost:3000".to_string()),
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert!(config.is_http());
    assert!(!config.is_stdio());
}

#[test]
fn test_server_config_with_environment_variables() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());

    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("node")),
        args: vec!["index.js".to_string()],
        env,
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.env.len(), 2);
}

#[test]
fn test_server_config_with_http_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let config = ServerConfig {
        name: "api-server".to_string(),
        transport: TransportType::Http,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: Some("https://api.example.com".to_string()),
        headers,
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.headers.len(), 2);
}

#[test]
fn test_server_config_unicode_name() {
    let config = ServerConfig {
        name: "\u{30b5}\u{30fc}\u{30d0}\u{30fc}_\u{1f680}".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("server")),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.name, "\u{30b5}\u{30fc}\u{30d0}\u{30fc}_\u{1f680}");
}

#[test]
fn test_server_config_very_long_name() {
    let long_name = "s".repeat(100_000);
    let config = ServerConfig {
        name: long_name.clone(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("server")),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.name.len(), 100_000);
}

#[test]
fn test_server_config_many_args() {
    let mut args = vec![];
    for i in 0..1000 {
        args.push(format!("--arg-{}", i));
    }

    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("cmd")),
        args,
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.args.len(), 1000);
}

#[test]
fn test_server_config_many_env_vars() {
    let mut env = HashMap::new();
    for i in 0..500 {
        env.insert(format!("VAR_{}", i), format!("value_{}", i));
    }

    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("cmd")),
        args: vec![],
        env,
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.env.len(), 500);
}

#[test]
fn test_server_config_clone() {
    let original = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Http,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: Some(PathBuf::from("/tmp")),
        url: Some("http://localhost:3000".to_string()),
        headers: HashMap::new(),
        default_timezone: Some("UTC".to_string()),
        default_city: Some("New York".to_string()),
    };

    let cloned = original.clone();
    assert_eq!(original, cloned);
}

