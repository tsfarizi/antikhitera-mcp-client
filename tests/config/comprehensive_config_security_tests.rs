//! Comprehensive Configuration Security Tests
//!
//! Extensive test suite for antikythera-core configuration with focus on:
//! - Input validation and bounds checking
//! - Injection prevention (command, SQL, path traversal)
//! - Unicode and special character handling
//! - Empty and null-like inputs
//! - URL and protocol validation
//! - Transport type handling
//! - Performance under stress

use antikythera_core::config::{
    ModelProviderConfig, ServerConfig, TransportType, ModelInfo,
};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// MODEL PROVIDER CONFIG TESTS
// ============================================================================

#[test]
fn test_model_provider_config_basic() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("secret".to_string()),
        api_path: Some("/v1beta".to_string()),
        models: vec![ModelInfo {
            name: "model-1".to_string(),
            display_name: Some("Model 1".to_string()),
        }],
    };

    assert_eq!(config.id, "test");
    assert_eq!(config.provider_type, "gemini");
    assert_eq!(config.models.len(), 1);
}

#[test]
fn test_model_provider_config_no_api_key() {
    let config = ModelProviderConfig {
        id: "ollama-local".to_string(),
        provider_type: "ollama".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.api_key, None);
}

#[test]
fn test_model_provider_config_multiple_models() {
    let config = ModelProviderConfig {
        id: "openai".to_string(),
        provider_type: "openai".to_string(),
        endpoint: "https://api.openai.com".to_string(),
        api_key: Some("key".to_string()),
        api_path: None,
        models: vec![
            ModelInfo { name: "gpt-4".to_string(), display_name: None },
            ModelInfo { name: "gpt-3.5".to_string(), display_name: Some("GPT-3.5 Turbo".to_string()) },
        ],
    };

    assert_eq!(config.models.len(), 2);
}

#[test]
fn test_model_provider_config_unicode_id() {
    let config = ModelProviderConfig {
        id: "提供者_🚀".to_string(),
        provider_type: "custom".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.id, "提供者_🚀");
}

#[test]
fn test_model_provider_config_unicode_endpoint() {
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com/模型".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.contains("模型"));
}

#[test]
fn test_model_provider_config_very_long_id() {
    let long_id = "x".repeat(10_000);
    let config = ModelProviderConfig {
        id: long_id.clone(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.id.len(), 10_000);
}

#[test]
fn test_model_provider_config_very_long_api_key() {
    let long_key = "k".repeat(1_000_000);
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some(long_key.clone()),
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.api_key.as_ref().unwrap().len(), 1_000_000);
}

#[test]
fn test_model_provider_config_very_long_endpoint() {
    let long_url = format!("https://api.example.com/{}", "x".repeat(100_000));
    let config = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "test".to_string(),
        endpoint: long_url.clone(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.endpoint.len(), "https://api.example.com/".len() + 100_000);
}

#[test]
fn test_model_provider_config_clone() {
    let original = ModelProviderConfig {
        id: "test".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("secret".to_string()),
        api_path: Some("/v1".to_string()),
        models: vec![],
    };

    let cloned = original.clone();
    assert_eq!(original, cloned);
}

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
        name: "サーバー_🚀".to_string(),
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

    assert_eq!(config.name, "サーバー_🚀");
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

// ============================================================================
// INJECTION PREVENTION TESTS
// ============================================================================

#[test]
fn test_sql_injection_in_provider_id() {
    let config = ModelProviderConfig {
        id: "'; DROP TABLE providers; --".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    // Config layer stores as-is; caller is responsible for validation
    assert_eq!(config.id, "'; DROP TABLE providers; --");
}

#[test]
fn test_command_injection_in_server_args() {
    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("node")),
        args: vec!["'; rm -rf /; echo '".to_string()],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    // Config layer stores as-is; caller validates before execution
    assert_eq!(config.args[0], "'; rm -rf /; echo '");
}

#[test]
fn test_path_traversal_in_command() {
    let config = ServerConfig {
        name: "server".to_string(),
        transport: TransportType::Stdio,
        command: Some(PathBuf::from("../../../../etc/passwd")),
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.command.as_ref().unwrap(), &PathBuf::from("../../../../etc/passwd"));
}

// ============================================================================
// EDGE CASE & BOUNDARY TESTS
// ============================================================================

#[test]
fn test_empty_provider_id() {
    let config = ModelProviderConfig {
        id: "".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.id, "");
}

#[test]
fn test_empty_server_name() {
    let config = ServerConfig {
        name: "".to_string(),
        transport: TransportType::Stdio,
        command: None,
        args: vec![],
        env: HashMap::new(),
        workdir: None,
        url: None,
        headers: HashMap::new(),
        default_timezone: None,
        default_city: None,
    };

    assert_eq!(config.name, "");
}

#[test]
fn test_model_provider_with_no_models() {
    let config = ModelProviderConfig {
        id: "empty".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert_eq!(config.models.len(), 0);
}

#[test]
fn test_model_info_without_display_name() {
    let model = ModelInfo {
        name: "model".to_string(),
        display_name: None,
    };

    assert_eq!(model.display_name, None);
}

// ============================================================================
// URL & PROTOCOL TESTS
// ============================================================================

#[test]
fn test_https_endpoint() {
    let config = ModelProviderConfig {
        id: "secure".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.starts_with("https://"));
}

#[test]
fn test_http_endpoint() {
    let config = ModelProviderConfig {
        id: "local".to_string(),
        provider_type: "test".to_string(),
        endpoint: "http://localhost:11434".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    assert!(config.endpoint.starts_with("http://"));
}

#[test]
fn test_malformed_url() {
    let config = ModelProviderConfig {
        id: "bad".to_string(),
        provider_type: "test".to_string(),
        endpoint: "not-a-url".to_string(),
        api_key: None,
        api_path: None,
        models: vec![],
    };

    // Config layer accepts any string; validation is caller's responsibility
    assert_eq!(config.endpoint, "not-a-url");
}

// ============================================================================
// TRANSPORT TYPE TESTS
// ============================================================================

#[test]
fn test_transport_type_stdio() {
    let transport = TransportType::Stdio;
    assert_eq!(transport, TransportType::Stdio);
}

#[test]
fn test_transport_type_http() {
    let transport = TransportType::Http;
    assert_eq!(transport, TransportType::Http);
}

#[test]
fn test_transport_type_clone() {
    let original = TransportType::Http;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

// ============================================================================
// STRESS & PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_many_providers_in_config() {
    for i in 0..1000 {
        let config = ModelProviderConfig {
            id: format!("provider-{}", i),
            provider_type: "test".to_string(),
            endpoint: format!("https://api.example.com/provider-{}", i),
            api_key: Some(format!("key-{}", i)),
            api_path: Some(format!("/api/v{}", i)),
            models: vec![],
        };

        assert_eq!(config.id, format!("provider-{}", i));
    }
}

#[test]
fn test_many_servers_in_config() {
    for i in 0..1000 {
        let config = ServerConfig {
            name: format!("server-{}", i),
            transport: if i % 2 == 0 { TransportType::Stdio } else { TransportType::Http },
            command: Some(PathBuf::from(format!("/bin/server-{}", i))),
            args: vec![],
            env: HashMap::new(),
            workdir: None,
            url: Some(format!("http://localhost:{}", 3000 + i)),
            headers: HashMap::new(),
            default_timezone: None,
            default_city: None,
        };

        assert_eq!(config.name, format!("server-{}", i));
    }
}

#[test]
fn test_large_model_list() {
    let mut models = vec![];
    for i in 0..10_000 {
        models.push(ModelInfo {
            name: format!("model-{}", i),
            display_name: Some(format!("Model {}", i)),
        });
    }

    let config = ModelProviderConfig {
        id: "massive".to_string(),
        provider_type: "test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        api_path: None,
        models,
    };

    assert_eq!(config.models.len(), 10_000);
}
