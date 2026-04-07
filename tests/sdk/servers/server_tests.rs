//! Server Management Integration Tests

use antikythera_sdk::servers::*;
use serial_test::serial;
use std::ffi::CString;

fn c_string_to_rust(ptr: *mut std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let s = std::ffi::CStr::from_ptr(ptr).to_str().unwrap().to_string();
        drop(CString::from_raw(ptr));
        s
    }
}

#[test]
#[serial]
fn test_add_valid_server() {
    servers_lock().clear();

    let config = McpServerConfig {
        name: "test-server".to_string(),
        transport: ServerTransport::Stdio,
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        env: vec![("NODE_ENV".to_string(), "production".to_string())],
        timeout_ms: Some(5000),
        enabled: true,
        description: Some("Test MCP Server".to_string()),
    };

    let json = serde_json::to_string(&config).unwrap();
    let c_json = CString::new(json).unwrap();

    let result_ptr = mcp_add_server(c_json.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

    assert!(validation.valid);
    assert_eq!(validation.server_name, "test-server");
    assert!(validation.errors.is_empty());
}

#[test]
#[serial]
fn test_add_duplicate_server() {
    servers_lock().clear();

    let config = McpServerConfig {
        name: "duplicate-server".to_string(),
        transport: ServerTransport::Stdio,
        command: "node".to_string(),
        args: vec![],
        env: vec![],
        timeout_ms: None,
        enabled: true,
        description: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let c_json = CString::new(json.clone()).unwrap();

    let result_ptr = mcp_add_server(c_json.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();
    assert!(validation.valid);

    let c_json2 = CString::new(json).unwrap();
    let result_ptr = mcp_add_server(c_json2.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

    assert!(!validation.valid);
    assert!(validation.errors.iter().any(|e| e.contains("already exists")));
}

#[test]
#[serial]
fn test_remove_server() {
    servers_lock().clear();

    let config = McpServerConfig {
        name: "to-remove".to_string(),
        transport: ServerTransport::Http,
        command: "http://localhost:3000".to_string(),
        args: vec![],
        env: vec![],
        timeout_ms: Some(3000),
        enabled: true,
        description: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    mcp_add_server(CString::new(json).unwrap().as_ptr());

    let name = CString::new("to-remove").unwrap();
    let result_ptr = mcp_remove_server(name.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let op_result: ServerOperationResult = serde_json::from_str(&result).unwrap();

    assert!(op_result.success);
    assert_eq!(op_result.server_name, "to-remove");
}

#[test]
#[serial]
fn test_server_validation_invalid_name() {
    let config = serde_json::json!({
        "name": "",
        "transport": "Stdio",
        "command": "",
        "args": [],
        "env": [],
        "timeout_ms": null,
        "enabled": true,
        "description": null
    });

    let json = config.to_string();
    let c_json = CString::new(json).unwrap();

    let result_ptr = mcp_validate_server(c_json.as_ptr());
    let result = c_string_to_rust(result_ptr);
    let validation: ServerValidationResult = serde_json::from_str(&result).unwrap();

    assert!(!validation.valid);
    assert!(!validation.errors.is_empty());
}

#[test]
#[serial]
fn test_export_import_servers() {
    servers_lock().clear();

    let config = McpServerConfig {
        name: "export-test".to_string(),
        transport: ServerTransport::Stdio,
        command: "test".to_string(),
        args: vec!["arg1".to_string()],
        env: vec![],
        timeout_ms: None,
        enabled: true,
        description: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    mcp_add_server(CString::new(json).unwrap().as_ptr());

    let export_ptr = mcp_export_servers_config();
    let export_json = c_string_to_rust(export_ptr);

    servers_lock().clear();

    let import_ptr = mcp_import_servers_config(CString::new(export_json).unwrap().as_ptr());
    let import_result = c_string_to_rust(import_ptr);
    let op_result: ServerOperationResult = serde_json::from_str(&import_result).unwrap();

    assert!(op_result.success);
    assert_eq!(op_result.tools_affected, 1);
}

#[test]
#[serial]
fn test_server_url_validation() {
    let config = McpServerConfig {
        name: "http-server".to_string(),
        transport: ServerTransport::Http,
        command: "not-a-url".to_string(),
        args: vec![],
        env: vec![],
        timeout_ms: None,
        enabled: true,
        description: None,
    };

    let validation = config.validate();
    assert!(!validation.valid);
    assert!(validation.errors.iter().any(|e| e.contains("URL")));
}
