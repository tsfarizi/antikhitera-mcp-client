//! Integration tests for DynamicComponent.

use antikhitera_mcp_client::domain::ui::DynamicComponent;
use serde_json::{Value, json};

#[test]
fn test_component_creation() {
    let component = DynamicComponent::new("product_card");
    assert_eq!(component.component_type, "product_card");
    assert!(component.props.is_empty());
    assert!(!component.has_children());
}

#[test]
fn test_builder_pattern() {
    let component = DynamicComponent::new("product_card")
        .with_prop("title", json!("iPhone 15"))
        .with_prop("price", json!(999.99))
        .with_prop("image", json!("base64..."));

    assert_eq!(component.get_string_prop("title"), Some("iPhone 15"));
    assert_eq!(component.get_f64_prop("price"), Some(999.99));
    assert_eq!(component.props.len(), 3);
}

#[test]
fn test_nested_container() {
    let card = DynamicComponent::new("product_card").with_prop("title", json!("Test"));

    let text = DynamicComponent::new("text").with_prop("content", json!("Analysis"));

    let container = DynamicComponent::new("container")
        .with_prop("direction", json!("horizontal"))
        .with_children(vec![card, text]);

    assert!(container.has_children());
    let children = container.children.as_ref().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].component_type, "product_card");
    assert_eq!(children[1].component_type, "text");
}

#[test]
fn test_serialization_format() {
    let component = DynamicComponent::new("text").with_prop("content", json!("Hello World"));

    let json_str = serde_json::to_string(&component).unwrap();
    let parsed: Value = serde_json::from_str(&json_str).unwrap();

    // Verify "type" and "id" fields are used
    assert_eq!(parsed["type"], "text");
    assert_eq!(parsed["id"], 0); // Default id
    assert_eq!(parsed["content"], "Hello World"); // Flattened
    // No children field when None
    assert!(parsed.get("children").is_none());
}

#[test]
fn test_deserialization() {
    let json_str = r#"{
        "type": "product_card",
        "title": "MacBook Pro",
        "price": 2499.99,
        "image": "base64encoded..."
    }"#;

    let component: DynamicComponent = serde_json::from_str(json_str).unwrap();
    assert_eq!(component.component_type, "product_card");
    assert_eq!(component.get_string_prop("title"), Some("MacBook Pro"));
    assert_eq!(component.get_f64_prop("price"), Some(2499.99));
}

#[test]
fn test_deep_nesting() {
    let leaf = DynamicComponent::new("text").with_prop("content", json!("Leaf"));

    let inner = DynamicComponent::new("container")
        .with_prop("direction", json!("vertical"))
        .with_children(vec![leaf]);

    let outer = DynamicComponent::new("container")
        .with_prop("direction", json!("horizontal"))
        .with_children(vec![inner]);

    assert!(outer.has_children());
    let inner_container = &outer.children.as_ref().unwrap()[0];
    assert!(inner_container.has_children());
}

#[test]
fn test_base64_image_passthrough() {
    // Simulate base64 image data from MCP
    let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    let component = DynamicComponent::new("product_card")
        .with_prop("title", json!("Test"))
        .with_prop("price", json!(99.99))
        .with_prop("image", json!(base64_image));

    // Verify base64 is preserved exactly
    assert_eq!(component.get_string_prop("image"), Some(base64_image));

    // Verify through serialization round-trip
    let json_str = serde_json::to_string(&component).unwrap();
    let restored: DynamicComponent = serde_json::from_str(&json_str).unwrap();
    assert_eq!(restored.get_string_prop("image"), Some(base64_image));
}
