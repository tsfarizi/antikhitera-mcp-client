//! Schema-driven UI assembler.
//!
//! Reads component definitions from TOML schema and hydrates
//! data from MCP tool execution results.

use super::AssemblerError;
use crate::application::agent::AgentStep;
use crate::domain::ui::{ComponentSchema, DynamicComponent, UiSchemaConfig};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

/// Schema-driven UI assembler.
///
/// Uses TOML schema as source of truth for field extraction and validation.
pub struct UiAssembler {
    schema: UiSchemaConfig,
    id_counter: AtomicI64,
}

impl UiAssembler {
    /// Create a new assembler with the given schema.
    pub fn new(schema: UiSchemaConfig) -> Self {
        Self {
            schema,
            id_counter: AtomicI64::new(1),
        }
    }

    /// Get the next unique ID for a component.
    fn next_id(&self) -> i64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Assemble UI response by hydrating a template with tool steps.
    ///
    /// # Search & Inject Logic
    /// - Recursively traverses the DynamicComponent tree.
    /// - If `data_source` is "step_N", extracts data from tool_steps[N].
    /// - Maps fields using `mapping` from ui.toml.
    /// - Performs strict type validation.
    pub fn assemble(
        &self,
        mut template: DynamicComponent,
        tool_steps: &[AgentStep],
    ) -> Result<DynamicComponent, AssemblerError> {
        self.hydrate_recursive(&mut template, tool_steps)?;
        Ok(template)
    }

    /// Recursively hydrate a component and its children.
    fn hydrate_recursive(
        &self,
        component: &mut DynamicComponent,
        tool_steps: &[AgentStep],
    ) -> Result<(), AssemblerError> {
        // 1. Assign ID if not set
        if component.id == 0 {
            component.id = self.next_id();
        }

        // 2. Hydrate if data_source is present
        if let Some(source) = &component.data_source {
            let index = parse_step_index(source)?;
            let step = tool_steps
                .get(index)
                .ok_or_else(|| AssemblerError::IndexOutOfBounds(index, tool_steps.len()))?;

            let schema = self
                .schema
                .get_component(&component.component_type)
                .ok_or_else(|| {
                    AssemblerError::UnknownComponent(component.component_type.clone())
                })?;

            let hydrated_props = self.extract_props(&step.output, schema)?;
            for (k, v) in hydrated_props {
                component.props.insert(k, v);
            }
        }

        // 3. Recurse into children
        if let Some(children) = &mut component.children {
            for child in children {
                self.hydrate_recursive(child, tool_steps)?;
            }
        }

        Ok(())
    }

    /// Extract props from tool output per schema's mapping roles.
    fn extract_props(
        &self,
        output: &Value,
        schema: &ComponentSchema,
    ) -> Result<HashMap<String, Value>, AssemblerError> {
        let mut props = HashMap::new();

        // Use mapping if available, otherwise fallback to old heuristic find_data_object
        if let Some(mapping) = &schema.mapping {
            let data = find_data_object(output)?;
            for (ui_field, data_path) in mapping {
                // Simplified JSONPath: $.field or field
                let key = data_path.trim_start_matches("$.").trim_start_matches('.');
                if let Some(value) = data.get(key) {
                    let expected_type = schema.get_field_type(ui_field).unwrap_or("any");
                    validate_type(ui_field, value, expected_type)?;
                    props.insert(ui_field.clone(), value.clone());
                } else if schema.is_required(ui_field) {
                    return Err(AssemblerError::MissingField(ui_field.clone()));
                }
            }
        } else {
            // Fallback for components without mapping (old logic)
            let data = find_data_object(output)?;
            for field in &schema.required_fields {
                let value = data
                    .get(field)
                    .ok_or_else(|| AssemblerError::MissingField(field.clone()))?;
                let expected_type = schema.get_field_type(field).unwrap_or("any");
                validate_type(field, value, expected_type)?;
                props.insert(field.clone(), value.clone());
            }
            for (field, expected_type) in &schema.optional_fields {
                if let Some(value) = data.get(field) {
                    validate_type(field, value, expected_type)?;
                    props.insert(field.clone(), value.clone());
                }
            }
        }

        Ok(props)
    }
}

/// Parse "step_N" into N.
fn parse_step_index(source: &str) -> Result<usize, AssemblerError> {
    if !source.starts_with("step_") {
        return Err(AssemblerError::InvalidStructure(format!(
            "Invalid data_source format: {}. Expected 'step_N'",
            source
        )));
    }
    source[5..]
        .parse::<usize>()
        .map_err(|_| AssemblerError::InvalidStructure(format!("Invalid step index in {}", source)))
}

/// Find data object in various tool output structures.
/// Priority: nested "data" > nested "product" > content array > direct object
fn find_data_object(output: &Value) -> Result<&Value, AssemblerError> {
    // Check nested "data" field first (highest priority)
    if let Some(data) = output.get("data").filter(|v| v.is_object()) {
        return Ok(data);
    }

    // Check nested "product" field
    if let Some(product) = output.get("product").filter(|v| v.is_object()) {
        return Ok(product);
    }

    // Check nested "structuredContent" field
    if let Some(sc) = output.get("structuredContent").filter(|v| v.is_object()) {
        return Ok(sc);
    }

    // Check content array
    if let Some(content) = output.get("content").and_then(Value::as_array) {
        for item in content {
            if item.is_object() {
                return Ok(item);
            }
        }
    }

    // Fall back to direct object (lowest priority)
    if output.is_object() {
        return Ok(output);
    }

    Err(AssemblerError::InvalidStructure(
        "No data object found in tool output".into(),
    ))
}

/// Validate value type against expected type from schema.
fn validate_type(field: &str, value: &Value, expected: &str) -> Result<(), AssemblerError> {
    let valid = match expected {
        "string" => value.is_string(),
        "f64" => value.is_f64() || value.is_i64(), // i64 can be converted to f64
        "i64" => value.is_i64(),
        "bool" => value.is_boolean(),
        "any" => true,
        _ => true, // Unknown types pass through
    };

    if !valid {
        return Err(AssemblerError::TypeError {
            field: field.into(),
            expected: expected.into(),
            actual: type_name(value),
        });
    }

    Ok(())
}

/// Get human-readable type name for a JSON value.
fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Number(n) if n.is_f64() => "f64".into(),
        Value::Number(_) => "i64".into(),
        Value::String(_) => "string".into(),
        Value::Array(_) => "array".into(),
        Value::Object(_) => "object".into(),
    }
}
