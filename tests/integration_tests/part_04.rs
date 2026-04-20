#[test]
#[ignore = "Requires custom MCP server running"]
fn test_custom_mcp_server() {
    // Check if custom server is running on port 8080
    require_server!("127.0.0.1", 8080);
    
    // If we reach here, server is running
    println!("âœ… MCP server available on port 8080, running test...");
    
    // Your test logic here (e.g., test MCP server integration)
    assert!(is_port_available("127.0.0.1", 8080));
}

/// Example test that requires all prerequisites
