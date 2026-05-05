use std::collections::HashMap;
use std::sync::atomic::Ordering;

use antikythera_core::application::tooling::{
    HttpTransport, HttpTransportConfig, McpTransport, ServerToolInfo, TransportMode,
};

#[tokio::test]
async fn test_http_transport_list_tools() {
    let config = HttpTransportConfig {
        name: "test".to_string(),
        url: "https://example.com/mcp".to_string(),
        headers: HashMap::new(),
        mode: TransportMode::Auto,
        required_capabilities: Vec::new(),
    };
    let transport = HttpTransport::new(config);

    // Manually populate cache
    {
        let mut cache = transport.inner.tool_cache.lock().await;
        cache.insert(
            "test_tool".to_string(),
            ServerToolInfo {
                name: "test_tool".to_string(),
                title: Some("Test Tool".to_string()),
                description: Some("test description".to_string()),
                icons: None,
                input_schema: None,
                output_schema: None,
                annotations: None,
                execution: None,
            },
        );
    }

    let tools = transport.list_tools().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "test_tool");
    assert_eq!(tools[0].title, Some("Test Tool".to_string()));
    assert_eq!(tools[0].description, Some("test description".to_string()));
}

#[tokio::test]
async fn test_http_transport_disconnect() {
    let config = HttpTransportConfig {
        name: "test".to_string(),
        url: "https://example.com/mcp".to_string(),
        headers: HashMap::new(),
        mode: TransportMode::Auto,
        required_capabilities: Vec::new(),
    };
    let transport = HttpTransport::new(config);
    transport.inner.connected.store(true, Ordering::SeqCst);

    // Populate cache
    {
        let mut cache = transport.inner.tool_cache.lock().await;
        cache.insert(
            "test_tool".to_string(),
            ServerToolInfo {
                name: "test_tool".to_string(),
                title: None,
                description: None,
                icons: None,
                input_schema: None,
                output_schema: None,
                annotations: None,
                execution: None,
            },
        );
    }

    transport.disconnect().await;

    assert!(!transport.is_connected().await);
    assert!(transport.list_tools().await.is_empty());
}
