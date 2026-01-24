//! Schema-driven UI assembler.
//!
//! Reads component definitions from TOML schema and hydrates
//! data from MCP tool execution results.

use super::AssemblerError;
use crate::application::agent::AgentStep;
use crate::domain::ui::{AgentLayoutIntent, ComponentSchema, DynamicComponent, UiSchemaConfig};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Schema-driven UI assembler.
///
/// Uses TOML schema as source of truth for field extraction and validation.
pub struct UiAssembler {
    schema: UiSchemaConfig,
}

impl UiAssembler {
    /// Create a new assembler with the given schema.
    pub fn new(schema: UiSchemaConfig) -> Self {
        Self { schema }
    }

    /// Assemble UI response from agent intent and tool steps.
    ///
    /// # Zero Hallucination Guarantee
    /// - Agent only provides `selected_data_index`
    /// - Actual data comes from tool_steps
    ///
    /// # Strict Type Check
    /// - Fields validated against `field_types` in TOML
    pub fn assemble(
        &self,
        intent: &AgentLayoutIntent,
        tool_steps: &[AgentStep],
    ) -> Result<DynamicComponent, AssemblerError> {
        // 1. Validate index bounds
        let step = tool_steps.get(intent.selected_data_index).ok_or_else(|| {
            AssemblerError::IndexOutOfBounds(intent.selected_data_index, tool_steps.len())
        })?;

        debug!(
            tool = %step.tool,
            index = intent.selected_data_index,
            component = %intent.component_type,
            "Assembling UI component from tool output"
        );

        // 2. Lookup component schema
        let component_schema = self
            .schema
            .get_component(&intent.component_type)
            .ok_or_else(|| AssemblerError::UnknownComponent(intent.component_type.clone()))?;

        // 3. Extract and validate props from tool output
        let props = self.extract_props(&step.output, component_schema)?;

        // 4. Build the data component
        let data_component = DynamicComponent {
            component_type: intent.component_type.clone(),
            props,
            children: None,
        };

        // 5. Build text component for analysis
        let text_component = DynamicComponent::new("text")
            .with_prop("content", Value::String(intent.analysis_text.clone()));

        // 6. Arrange children based on position
        let children = if intent.card_first() {
            vec![data_component, text_component]
        } else {
            vec![text_component, data_component]
        };

        // 7. Build container
        Ok(DynamicComponent::new("container")
            .with_prop("direction", Value::String(intent.layout_direction.clone()))
            .with_children(children))
    }

    /// Extract props from tool output per schema.
    fn extract_props(
        &self,
        output: &Value,
        schema: &ComponentSchema,
    ) -> Result<HashMap<String, Value>, AssemblerError> {
        let data = find_data_object(output)?;
        let mut props = HashMap::new();

        // Extract and validate required fields
        for field in &schema.required_fields {
            let value = data
                .get(field)
                .ok_or_else(|| AssemblerError::MissingField(field.clone()))?;

            let expected_type = schema.get_field_type(field).unwrap_or("any");
            validate_type(field, value, expected_type)?;

            props.insert(field.clone(), value.clone());
        }

        // Extract optional fields if present
        for (field, expected_type) in &schema.optional_fields {
            if let Some(value) = data.get(field) {
                validate_type(field, value, expected_type)?;
                props.insert(field.clone(), value.clone());
            }
        }

        Ok(props)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_schema() -> UiSchemaConfig {
        toml::from_str(
            r#"
            [components.product_card]
            required_fields = ["title", "price", "image"]
            field_types = { title = "string", price = "f64", image = "string" }
            optional_fields = { is_discounted = "bool" }

            [components.text]
            required_fields = ["content"]
            field_types = { content = "string" }

            [components.container]
            required_fields = ["direction"]
            field_types = { direction = "string" }
            is_container = true
        "#,
        )
        .unwrap()
    }

    fn mock_step(output: Value) -> AgentStep {
        AgentStep {
            tool: "test_tool".into(),
            input: json!({}),
            success: true,
            output,
            message: None,
        }
    }

    #[test]
    fn test_assemble_basic_layout() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Great product!".into(),
            selected_data_index: 0,
            component_type: "product_card".into(),
            layout_direction: "horizontal".into(),
            card_position: "left".into(),
        };

        let steps = vec![mock_step(json!({
            "title": "Test Product",
            "price": 99.99,
            "image": "base64data..."
        }))];

        let result = assembler.assemble(&intent, &steps).unwrap();

        assert_eq!(result.component_type, "container");
        assert_eq!(result.get_string_prop("direction"), Some("horizontal"));
        assert!(result.has_children());

        let children = result.children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].component_type, "product_card"); // left = first
        assert_eq!(children[1].component_type, "text");
    }

    #[test]
    fn test_reject_string_price() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Test".into(),
            selected_data_index: 0,
            component_type: "product_card".into(),
            layout_direction: "vertical".into(),
            card_position: "top".into(),
        };

        let steps = vec![mock_step(json!({
            "title": "Test",
            "price": "99.99",  // String price - should be rejected!
            "image": "base64..."
        }))];

        let result = assembler.assemble(&intent, &steps);
        assert!(matches!(result, Err(AssemblerError::TypeError { .. })));
    }

    #[test]
    fn test_index_out_of_bounds() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Test".into(),
            selected_data_index: 5,
            component_type: "product_card".into(),
            layout_direction: "vertical".into(),
            card_position: "top".into(),
        };

        let result = assembler.assemble(&intent, &[]);
        assert!(matches!(
            result,
            Err(AssemblerError::IndexOutOfBounds(5, 0))
        ));
    }

    #[test]
    fn test_unknown_component() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Test".into(),
            selected_data_index: 0,
            component_type: "unknown_widget".into(),
            layout_direction: "vertical".into(),
            card_position: "top".into(),
        };

        let steps = vec![mock_step(json!({"foo": "bar"}))];
        let result = assembler.assemble(&intent, &steps);
        assert!(matches!(result, Err(AssemblerError::UnknownComponent(_))));
    }

    #[test]
    fn test_missing_required_field() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Test".into(),
            selected_data_index: 0,
            component_type: "product_card".into(),
            layout_direction: "vertical".into(),
            card_position: "top".into(),
        };

        let steps = vec![mock_step(json!({
            "title": "Test",
            // missing price and image
        }))];

        let result = assembler.assemble(&intent, &steps);
        assert!(matches!(result, Err(AssemblerError::MissingField(_))));
    }

    #[test]
    fn test_optional_field_included() {
        let assembler = UiAssembler::new(test_schema());

        let intent = AgentLayoutIntent {
            analysis_text: "Sale!".into(),
            selected_data_index: 0,
            component_type: "product_card".into(),
            layout_direction: "horizontal".into(),
            card_position: "right".into(),
        };

        let steps = vec![mock_step(json!({
            "title": "On Sale",
            "price": 49.99,
            "image": "base64...",
            "is_discounted": true
        }))];

        let result = assembler.assemble(&intent, &steps).unwrap();
        let children = result.children.as_ref().unwrap();
        let card = &children[1]; // right = second

        assert_eq!(card.get_prop("is_discounted"), Some(&Value::Bool(true)));
    }
}
