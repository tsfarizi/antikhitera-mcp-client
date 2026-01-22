//! TOML schema loader for UI components.
//!
//! Reads component definitions from `config/ui.toml` as source of truth.

use serde::Deserialize;
use std::collections::HashMap;

/// Schema definition for a single component from ui.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSchema {
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Fields that MUST be present in MCP output
    #[serde(default)]
    pub required_fields: Vec<String>,

    /// Type definitions for validation: field_name -> type_name
    /// Supported types: "string", "f64", "i64", "bool"
    #[serde(default)]
    pub field_types: HashMap<String, String>,

    /// Optional fields with their types
    #[serde(default)]
    pub optional_fields: HashMap<String, String>,

    /// Whether this component can have children
    #[serde(default)]
    pub is_container: bool,
}

impl ComponentSchema {
    /// Get the expected type for a field.
    pub fn get_field_type(&self, field: &str) -> Option<&str> {
        self.field_types
            .get(field)
            .or_else(|| self.optional_fields.get(field))
            .map(String::as_str)
    }

    /// Check if a field is required.
    pub fn is_required(&self, field: &str) -> bool {
        self.required_fields.contains(&field.to_string())
    }

    /// Get all field names (required + optional).
    pub fn all_fields(&self) -> impl Iterator<Item = &str> {
        self.required_fields
            .iter()
            .map(String::as_str)
            .chain(self.optional_fields.keys().map(String::as_str))
    }
}

/// Root config structure for ui.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct UiSchemaConfig {
    /// Component definitions keyed by name
    #[serde(default)]
    pub components: HashMap<String, ComponentSchema>,
}

impl UiSchemaConfig {
    /// Get schema for a component by name.
    pub fn get_component(&self, name: &str) -> Option<&ComponentSchema> {
        self.components.get(name)
    }

    /// Check if a component exists.
    pub fn has_component(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// List all component names.
    pub fn component_names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }
}

impl Default for UiSchemaConfig {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_component_schema() {
        let toml = r#"
            [components.product_card]
            description = "Product display card"
            required_fields = ["title", "price", "image"]
            field_types = { title = "string", price = "f64", image = "string" }
            optional_fields = { is_discounted = "bool" }

            [components.container]
            required_fields = ["direction"]
            field_types = { direction = "string" }
            is_container = true
        "#;

        let config: UiSchemaConfig = toml::from_str(toml).unwrap();

        let product = config.get_component("product_card").unwrap();
        assert_eq!(product.required_fields.len(), 3);
        assert_eq!(product.get_field_type("price"), Some("f64"));
        assert!(product.is_required("title"));
        assert!(!product.is_container);

        let container = config.get_component("container").unwrap();
        assert!(container.is_container);
    }

    #[test]
    fn test_all_fields() {
        let schema = ComponentSchema {
            description: None,
            required_fields: vec!["a".into(), "b".into()],
            field_types: [("a".into(), "string".into()), ("b".into(), "i64".into())]
                .into_iter()
                .collect(),
            optional_fields: [("c".into(), "bool".into())].into_iter().collect(),
            is_container: false,
        };

        let fields: Vec<_> = schema.all_fields().collect();
        assert!(fields.contains(&"a"));
        assert!(fields.contains(&"b"));
        assert!(fields.contains(&"c"));
    }
}
