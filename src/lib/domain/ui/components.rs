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
    pub component_type: String,

    /// Unique session-based incrementing ID.
    pub id: i64,

    /// Dynamic properties hydrated from MCP data per TOML schema.
    /// Keys must match `required_fields` + `optional_fields` from schema.
    #[schema(value_type = Object)]
    #[serde(flatten)]
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
            component_type: name.into(),
            id: 0,
            props: HashMap::new(),
            children: None,
        }
    }

    /// Set the unique ID for this component.
    pub fn with_id(mut self, id: i64) -> Self {
        self.id = id;
        self
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
