//! Integration tests for UiAssembler.

use antikhitera_mcp_client::application::agent::AgentStep;
use antikhitera_mcp_client::application::ui::{AssemblerError, UiAssembler};
use antikhitera_mcp_client::domain::ui::{AgentLayoutIntent, UiSchemaConfig};
use serde_json::{Value, json};

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

        [components.rating_display]
        required_fields = ["rating", "review_count"]
        field_types = { rating = "f64", review_count = "i64" }
    "#,
    )
    .unwrap()
}

fn mock_step(output: Value) -> AgentStep {
    AgentStep {
        tool: "mock_tool".into(),
        input: json!({}),
        success: true,
        output,
        message: None,
    }
}

fn basic_intent() -> AgentLayoutIntent {
    AgentLayoutIntent {
        analysis_text: "Great product recommendation!".into(),
        selected_data_index: 0,
        component_type: "product_card".into(),
        layout_direction: "horizontal".into(),
        card_position: "left".into(),
    }
}

// =============================================================================
// SUCCESS CASES
// =============================================================================

#[test]
fn test_assemble_horizontal_left_layout() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "iPhone 15 Pro",
        "price": 1199.99,
        "image": "base64imagedata..."
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();

    assert_eq!(result.component_type, "container");
    assert_eq!(result.get_string_prop("direction"), Some("horizontal"));

    let children = result.children.as_ref().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].component_type, "product_card"); // left = first
    assert_eq!(children[1].component_type, "text");
}

#[test]
fn test_assemble_vertical_bottom_layout() {
    let assembler = UiAssembler::new(test_schema());
    let intent = AgentLayoutIntent {
        card_position: "bottom".into(),
        layout_direction: "vertical".into(),
        ..basic_intent()
    };
    let steps = vec![mock_step(json!({
        "title": "Test",
        "price": 99.99,
        "image": "base64..."
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();

    assert_eq!(result.get_string_prop("direction"), Some("vertical"));
    let children = result.children.as_ref().unwrap();
    assert_eq!(children[0].component_type, "text"); // bottom = text first
    assert_eq!(children[1].component_type, "product_card");
}

#[test]
fn test_assemble_with_optional_fields() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Sale Item",
        "price": 49.99,
        "image": "base64...",
        "is_discounted": true
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];

    assert_eq!(card.get_prop("is_discounted"), Some(&Value::Bool(true)));
}

#[test]
fn test_assemble_with_nested_data() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    // Data nested in "data" field
    let steps = vec![mock_step(json!({
        "data": {
            "title": "Nested Product",
            "price": 199.99,
            "image": "base64..."
        }
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];
    assert_eq!(card.get_string_prop("title"), Some("Nested Product"));
}

#[test]
fn test_assemble_with_product_nested_data() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    // Data nested in "product" field
    let steps = vec![mock_step(json!({
        "product": {
            "title": "Product Nested",
            "price": 299.99,
            "image": "base64..."
        }
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];
    assert_eq!(card.get_string_prop("title"), Some("Product Nested"));
}

#[test]
fn test_integer_price_accepted() {
    // Integer prices should be accepted (can convert to f64)
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Test",
        "price": 100,  // Integer, not float
        "image": "base64..."
    }))];

    let result = assembler.assemble(&intent, &steps);
    assert!(result.is_ok());
}

// =============================================================================
// ERROR CASES
// =============================================================================

#[test]
fn test_error_index_out_of_bounds() {
    let assembler = UiAssembler::new(test_schema());
    let intent = AgentLayoutIntent {
        selected_data_index: 5,
        ..basic_intent()
    };

    let result = assembler.assemble(&intent, &[]);

    match result {
        Err(AssemblerError::IndexOutOfBounds(idx, len)) => {
            assert_eq!(idx, 5);
            assert_eq!(len, 0);
        }
        _ => panic!("Expected IndexOutOfBounds error"),
    }
}

#[test]
fn test_error_unknown_component() {
    let assembler = UiAssembler::new(test_schema());
    let intent = AgentLayoutIntent {
        component_type: "unknown_widget".into(),
        ..basic_intent()
    };
    let steps = vec![mock_step(json!({"foo": "bar"}))];

    let result = assembler.assemble(&intent, &steps);

    match result {
        Err(AssemblerError::UnknownComponent(name)) => {
            assert_eq!(name, "unknown_widget");
        }
        _ => panic!("Expected UnknownComponent error"),
    }
}

#[test]
fn test_error_missing_required_field() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Test"
        // Missing price and image
    }))];

    let result = assembler.assemble(&intent, &steps);

    match result {
        Err(AssemblerError::MissingField(field)) => {
            assert!(field == "price" || field == "image");
        }
        _ => panic!("Expected MissingField error"),
    }
}

#[test]
fn test_error_string_price_rejected() {
    // String prices MUST be rejected (strict type check)
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Test",
        "price": "99.99",  // String, not number!
        "image": "base64..."
    }))];

    let result = assembler.assemble(&intent, &steps);

    match result {
        Err(AssemblerError::TypeError {
            field,
            expected,
            actual,
        }) => {
            assert_eq!(field, "price");
            assert_eq!(expected, "f64");
            assert_eq!(actual, "string");
        }
        _ => panic!("Expected TypeError for string price"),
    }
}

#[test]
fn test_error_wrong_type_for_boolean() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Test",
        "price": 99.99,
        "image": "base64...",
        "is_discounted": "yes"  // String instead of bool
    }))];

    let result = assembler.assemble(&intent, &steps);

    match result {
        Err(AssemblerError::TypeError {
            field, expected, ..
        }) => {
            assert_eq!(field, "is_discounted");
            assert_eq!(expected, "bool");
        }
        _ => panic!("Expected TypeError for boolean field"),
    }
}

#[test]
fn test_error_invalid_structure() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    // Array instead of object
    let steps = vec![mock_step(json!(["not", "an", "object"]))];

    let result = assembler.assemble(&intent, &steps);
    assert!(matches!(result, Err(AssemblerError::InvalidStructure(_))));
}

// =============================================================================
// BASE64 INTEGRITY
// =============================================================================

#[test]
fn test_base64_passthrough_integrity() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();

    // Real PNG 1x1 pixel base64
    let original_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    let steps = vec![mock_step(json!({
        "title": "Image Test",
        "price": 99.99,
        "image": original_base64
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];

    // Verify base64 is EXACTLY preserved
    assert_eq!(card.get_string_prop("image"), Some(original_base64));
}

#[test]
fn test_large_base64_passthrough() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();

    // Simulate larger base64 data
    let large_base64: String = "A".repeat(10000);

    let steps = vec![mock_step(json!({
        "title": "Large Image",
        "price": 99.99,
        "image": large_base64
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];

    assert_eq!(card.get_string_prop("image").unwrap().len(), 10000);
}

// =============================================================================
// MULTIPLE STEPS
// =============================================================================

#[test]
fn test_select_specific_step() {
    let assembler = UiAssembler::new(test_schema());
    let intent = AgentLayoutIntent {
        selected_data_index: 1, // Select second step
        ..basic_intent()
    };

    let steps = vec![
        mock_step(json!({
            "title": "First Product",
            "price": 100.0,
            "image": "first_base64"
        })),
        mock_step(json!({
            "title": "Second Product",
            "price": 200.0,
            "image": "second_base64"
        })),
        mock_step(json!({
            "title": "Third Product",
            "price": 300.0,
            "image": "third_base64"
        })),
    ];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let card = &result.children.as_ref().unwrap()[0];

    // Should get data from step index 1
    assert_eq!(card.get_string_prop("title"), Some("Second Product"));
    assert_eq!(card.get_f64_prop("price"), Some(200.0));
}

#[test]
fn test_structured_content_discovery() {
    let assembler = UiAssembler::new(test_schema());

    let intent = AgentLayoutIntent {
        analysis_text: "Discovery!".into(),
        selected_data_index: 0,
        component_type: "product_card".into(),
        layout_direction: "vertical".into(),
        card_position: "top".into(),
    };

    let steps = vec![mock_step(json!({
        "structuredContent": {
            "title": "Discovery Card",
            "price": 10.0,
            "image": "img..."
        }
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();
    let children = result.children.as_ref().unwrap();
    let card = &children[0];

    assert_eq!(card.get_string_prop("title"), Some("Discovery Card"));
}

#[test]
fn test_id_incrementing() {
    let assembler = UiAssembler::new(test_schema());
    let intent = basic_intent();
    let steps = vec![mock_step(json!({
        "title": "Test",
        "price": 10.0,
        "image": "img"
    }))];

    let result = assembler.assemble(&intent, &steps).unwrap();

    // Result is a container with text and product_card children
    // container id should be 3 (1: card, 2: text, 3: container)
    // Wait, let's trace IDs:
    // 1. data_component (id = 1)
    // 2. text_component (id = 2)
    // 3. container (id = 3)

    assert_eq!(result.id, 3);
    assert_eq!(result.component_type, "container");

    let children = result.children.as_ref().unwrap();
    // card_first defaults to true for horizontal/left
    assert_eq!(children[0].id, 1);
    assert_eq!(children[1].id, 2);

    // Another assembly should continue the counter
    let result2 = assembler.assemble(&intent, &steps).unwrap();
    assert_eq!(result2.id, 6);
    let children2 = result2.children.as_ref().unwrap();
    assert_eq!(children2[0].id, 4);
    assert_eq!(children2[1].id, 5);
}
