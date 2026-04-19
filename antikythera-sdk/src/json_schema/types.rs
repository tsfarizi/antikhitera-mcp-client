//! JSON Schema Type System
//!
//! Defines expected JSON output structure with nested types for validation.
//! Supports: String, Number, Boolean, Array, Object (with nested fields)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Type Definitions
// ============================================================================

/// Supported data types for JSON schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchemaType {
    /// String value
    String,
    /// Integer value
    Integer,
    /// Floating point number
    Float,
    /// Boolean value
    Boolean,
    /// Array of items with specified type
    Array { items: Box<SchemaType> },
    /// Object with nested fields
    Object {
        fields: HashMap<String, SchemaField>,
    },
}

impl PartialEq for SchemaType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SchemaType::String, SchemaType::String) => true,
            (SchemaType::Integer, SchemaType::Integer) => true,
            (SchemaType::Float, SchemaType::Float) => true,
            (SchemaType::Boolean, SchemaType::Boolean) => true,
            (SchemaType::Array { items: a }, SchemaType::Array { items: b }) => a == b,
            (SchemaType::Object { fields: a }, SchemaType::Object { fields: b }) => a == b,
            _ => false,
        }
    }
}

impl SchemaType {
    /// Get human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self {
            SchemaType::String => "string",
            SchemaType::Integer => "integer",
            SchemaType::Float => "float",
            SchemaType::Boolean => "boolean",
            SchemaType::Array { .. } => "array",
            SchemaType::Object { .. } => "object",
        }
    }

    /// Check if a serde_json::Value matches this type
    pub fn matches_value(&self, value: &serde_json::Value) -> bool {
        match (self, value) {
            (SchemaType::String, serde_json::Value::String(_)) => true,
            (SchemaType::Integer, serde_json::Value::Number(n)) => n.is_i64(),
            (SchemaType::Float, serde_json::Value::Number(n)) => n.is_f64(),
            (SchemaType::Boolean, serde_json::Value::Bool(_)) => true,
            (SchemaType::Array { items }, serde_json::Value::Array(arr)) => {
                arr.iter().all(|v| items.matches_value(v))
            }
            (SchemaType::Object { fields }, serde_json::Value::Object(obj)) => {
                fields.iter().all(|(key, field)| {
                    if field.required {
                        obj.contains_key(key) && field.field_type.matches_value(&obj[key])
                    } else {
                        obj.get(key)
                            .map(|v| field.field_type.matches_value(v))
                            .unwrap_or(true)
                    }
                })
            }
            _ => false,
        }
    }
}

/// Schema field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    /// Field type
    #[serde(flatten)]
    pub field_type: SchemaType,
    /// Whether this field is required
    #[serde(default = "default_true")]
    pub required: bool,
    /// Field description for documentation
    pub description: Option<String>,
    /// Example value
    pub example: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl PartialEq for SchemaField {
    fn eq(&self, other: &Self) -> bool {
        self.field_type == other.field_type
            && self.required == other.required
            && self.description == other.description
    }
}

// ============================================================================
// JSON Schema Definition
// ============================================================================

/// Complete JSON schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// Schema name for identification
    pub name: String,
    /// Root type definition
    #[serde(flatten)]
    pub root_type: SchemaType,
    /// Description of expected structure
    pub description: Option<String>,
    /// Whether to allow additional fields not in schema
    #[serde(default)]
    pub allow_additional: bool,
}

impl JsonSchema {
    /// Validate a JSON string against this schema
    pub fn validate(&self, json_str: &str) -> Result<(), ValidationError> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| ValidationError::InvalidJson(e.to_string()))?;

        if !self.root_type.matches_value(&value) {
            return Err(ValidationError::TypeMismatch {
                expected: self.root_type.type_name().to_string(),
                got: value_type_name(&value),
                path: "$".to_string(),
            });
        }

        // Check required fields for objects
        if let (SchemaType::Object { fields }, serde_json::Value::Object(obj)) =
            (&self.root_type, &value)
        {
            for (key, field) in fields {
                if field.required && !obj.contains_key(key) {
                    return Err(ValidationError::MissingRequiredField {
                        field: key.clone(),
                        path: "$".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Generate example JSON from schema
    pub fn generate_example(&self) -> String {
        serde_json::to_string_pretty(&generate_example_value(&self.root_type))
            .unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate prompt instruction for LLM
    pub fn to_prompt_instruction(&self) -> String {
        let mut instruction = String::new();

        instruction.push_str("You must respond with a JSON object matching this schema:\n");
        instruction.push_str(&format!("Schema: {}\n", self.name));

        if let Some(desc) = &self.description {
            instruction.push_str(&format!("Description: {}\n", desc));
        }

        instruction.push_str("\nRequired structure:\n");
        instruction.push_str(&generate_schema_instruction(&self.root_type, 0));

        instruction.push_str("\n\nExample output:\n```json\n");
        instruction.push_str(&self.generate_example());
        instruction.push_str("\n```\n");

        instruction
            .push_str("\nIMPORTANT: Respond with ONLY valid JSON. No explanations or markdown.");

        instruction
    }
}

// ============================================================================
// Validation Errors
// ============================================================================

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    /// JSON parsing failed
    InvalidJson(String),
    /// Type mismatch
    TypeMismatch {
        expected: String,
        got: String,
        path: String,
    },
    /// Required field missing
    MissingRequiredField { field: String, path: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidJson(e) => write!(f, "Invalid JSON: {}", e),
            ValidationError::TypeMismatch {
                expected,
                got,
                path,
            } => {
                write!(
                    f,
                    "Type mismatch at {}: expected {}, got {}",
                    path, expected, got
                )
            }
            ValidationError::MissingRequiredField { field, path } => {
                write!(f, "Missing required field '{}' at {}", field, path)
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn value_type_name(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(n) => if n.is_i64() { "integer" } else { "float" }.to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

fn generate_example_value(schema_type: &SchemaType) -> serde_json::Value {
    match schema_type {
        SchemaType::String => serde_json::json!("example_string"),
        SchemaType::Integer => serde_json::json!(0),
        SchemaType::Float => serde_json::json!(0.0),
        SchemaType::Boolean => serde_json::json!(true),
        SchemaType::Array { items } => {
            serde_json::json!([generate_example_value(items)])
        }
        SchemaType::Object { fields } => {
            let mut obj = serde_json::Map::new();
            for (key, field) in fields {
                obj.insert(key.clone(), generate_example_value(&field.field_type));
            }
            serde_json::Value::Object(obj)
        }
    }
}

fn generate_schema_instruction(schema_type: &SchemaType, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);

    match schema_type {
        SchemaType::String => format!("{}(string)\n", indent_str),
        SchemaType::Integer => format!("{}(integer)\n", indent_str),
        SchemaType::Float => format!("{}(float)\n", indent_str),
        SchemaType::Boolean => format!("{}(boolean)\n", indent_str),
        SchemaType::Array { items } => {
            let mut s = format!("{}array of:\n", indent_str);
            s.push_str(&generate_schema_instruction(items, indent + 1));
            s
        }
        SchemaType::Object { fields } => {
            let mut s = format!("{}object:\n", indent_str);
            for (key, field) in fields {
                let req = if field.required {
                    "(required)"
                } else {
                    "(optional)"
                };
                s.push_str(&format!("{}{} {}:\n", indent_str, key, req));
                if let Some(desc) = &field.description {
                    s.push_str(&format!("{}  # {}\n", indent_str, desc));
                }
                s.push_str(&generate_schema_instruction(&field.field_type, indent + 2));
            }
            s
        }
    }
}

// ============================================================================
// Common Schema Builders
// ============================================================================

impl JsonSchema {
    /// Create a simple string field schema
    pub fn string_field(_name: &str, required: bool, description: Option<&str>) -> SchemaField {
        SchemaField {
            field_type: SchemaType::String,
            required,
            description: description.map(|s| s.to_string()),
            example: None,
        }
    }

    /// Create a simple integer field schema
    pub fn int_field(_name: &str, required: bool, description: Option<&str>) -> SchemaField {
        SchemaField {
            field_type: SchemaType::Integer,
            required,
            description: description.map(|s| s.to_string()),
            example: None,
        }
    }

    /// Create a simple boolean field schema
    pub fn bool_field(_name: &str, required: bool, description: Option<&str>) -> SchemaField {
        SchemaField {
            field_type: SchemaType::Boolean,
            required,
            description: description.map(|s| s.to_string()),
            example: None,
        }
    }

    /// Create a nested object field schema
    pub fn object_field(
        _name: &str,
        fields: HashMap<String, SchemaField>,
        required: bool,
        description: Option<&str>,
    ) -> SchemaField {
        SchemaField {
            field_type: SchemaType::Object { fields },
            required,
            description: description.map(|s| s.to_string()),
            example: None,
        }
    }

    /// Create an array field schema
    pub fn array_field(
        _name: &str,
        item_type: SchemaType,
        required: bool,
        description: Option<&str>,
    ) -> SchemaField {
        SchemaField {
            field_type: SchemaType::Array {
                items: Box::new(item_type),
            },
            required,
            description: description.map(|s| s.to_string()),
            example: None,
        }
    }
}
