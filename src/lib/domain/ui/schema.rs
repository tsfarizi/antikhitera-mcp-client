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

    /// Mapping between UI slots and MCP output keys (e.g. title = "$.name")
    #[serde(default)]
    pub mapping: Option<HashMap<String, String>>,
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

    /// Get schema for a component by name (alias for get_component).
    pub fn get_component_schema(&self, name: &str) -> Option<&ComponentSchema> {
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
