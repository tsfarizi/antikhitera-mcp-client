//! Dynamic UI components - no hard-coded struct definitions.
//!
//! Uses HashMap<String, Value> for props, validated against TOML schema.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Generic dynamic component - schema-driven, no struct definitions.
///
/// The `component_name` references a definition in `config/ui.toml`,
/// and `props` are validated against that schema's `field_types`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DynamicComponent {
    /// Component name from ui.toml (e.g. "product_card", "text", "container")
    #[serde(rename = "type")]
    pub component_name: String,

    /// Dynamic properties hydrated from MCP data per TOML schema.
    /// Keys must match `required_fields` + `optional_fields` from schema.
    #[schema(value_type = Object)]
    pub props: HashMap<String, Value>,

    /// Optional children for container components.
    /// Only valid when schema has `is_container = true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DynamicComponent>>,
}

impl DynamicComponent {
    /// Create a new component with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            component_name: name.into(),
            props: HashMap::new(),
            children: None,
        }
    }

    /// Add a property to this component.
    pub fn with_prop(mut self, key: &str, value: Value) -> Self {
        self.props.insert(key.to_string(), value);
        self
    }

    /// Set children for container components.
    pub fn with_children(mut self, children: Vec<DynamicComponent>) -> Self {
        self.children = Some(children);
        self
    }

    /// Check if this component has children.
    pub fn has_children(&self) -> bool {
        self.children.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// Get a prop value by key.
    pub fn get_prop(&self, key: &str) -> Option<&Value> {
        self.props.get(key)
    }

    /// Get a prop as string.
    pub fn get_string_prop(&self, key: &str) -> Option<&str> {
        self.props.get(key).and_then(Value::as_str)
    }

    /// Get a prop as f64.
    pub fn get_f64_prop(&self, key: &str) -> Option<f64> {
        self.props.get(key).and_then(Value::as_f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dynamic_component_builder() {
        let component = DynamicComponent::new("product_card")
            .with_prop("title", json!("Test Product"))
            .with_prop("price", json!(99.99))
            .with_prop("image", json!("base64data..."));

        assert_eq!(component.component_name, "product_card");
        assert_eq!(component.get_string_prop("title"), Some("Test Product"));
        assert_eq!(component.get_f64_prop("price"), Some(99.99));
        assert!(!component.has_children());
    }

    #[test]
    fn test_container_with_children() {
        let child = DynamicComponent::new("text").with_prop("content", json!("Hello"));

        let container = DynamicComponent::new("container")
            .with_prop("direction", json!("horizontal"))
            .with_children(vec![child]);

        assert!(container.has_children());
        assert_eq!(container.children.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_serialization() {
        let component = DynamicComponent::new("text").with_prop("content", json!("Hello World"));

        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"content\":\"Hello World\""));
    }
}
