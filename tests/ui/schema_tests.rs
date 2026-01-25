//! Integration tests for UiSchemaConfig.

use antikhitera_mcp_client::domain::ui::UiSchemaConfig;

fn load_test_schema() -> UiSchemaConfig {
    toml::from_str(
        r#"
        [components.product_card]
        description = "Product display card"
        required_fields = ["title", "price", "image"]
        field_types = { title = "string", price = "f64", image = "string" }
        optional_fields = { is_discounted = "bool", discount_percent = "f64" }

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
    .expect("Failed to parse test schema")
}

#[test]
fn test_schema_loading() {
    let schema = load_test_schema();
    assert!(schema.has_component("product_card"));
    assert!(schema.has_component("text"));
    assert!(schema.has_component("container"));
    assert!(!schema.has_component("unknown"));
}

#[test]
fn test_component_names() {
    let schema = load_test_schema();
    let names: Vec<_> = schema.component_names().collect();
    assert!(names.contains(&"product_card"));
    assert!(names.contains(&"text"));
    assert!(names.contains(&"container"));
    assert!(names.contains(&"rating_display"));
}

#[test]
fn test_required_fields() {
    let schema = load_test_schema();
    let product = schema.get_component("product_card").unwrap();

    assert!(product.is_required("title"));
    assert!(product.is_required("price"));
    assert!(product.is_required("image"));
    assert!(!product.is_required("is_discounted"));
}

#[test]
fn test_field_types() {
    let schema = load_test_schema();
    let product = schema.get_component("product_card").unwrap();

    assert_eq!(product.get_field_type("title"), Some("string"));
    assert_eq!(product.get_field_type("price"), Some("f64"));
    assert_eq!(product.get_field_type("is_discounted"), Some("bool"));
    assert_eq!(product.get_field_type("unknown"), None);
}

#[test]
fn test_optional_fields() {
    let schema = load_test_schema();
    let product = schema.get_component("product_card").unwrap();

    assert!(!product.optional_fields.is_empty());
    assert!(product.optional_fields.contains_key("is_discounted"));
    assert!(product.optional_fields.contains_key("discount_percent"));
}

#[test]
fn test_container_flag() {
    let schema = load_test_schema();

    let container = schema.get_component("container").unwrap();
    assert!(container.is_container);

    let product = schema.get_component("product_card").unwrap();
    assert!(!product.is_container);
}

#[test]
fn test_all_fields_iterator() {
    let schema = load_test_schema();
    let product = schema.get_component("product_card").unwrap();

    let all_fields: Vec<_> = product.all_fields().collect();
    assert!(all_fields.contains(&"title"));
    assert!(all_fields.contains(&"price"));
    assert!(all_fields.contains(&"image"));
    assert!(all_fields.contains(&"is_discounted"));
    assert!(all_fields.contains(&"discount_percent"));
}

#[test]
fn test_rating_display_integer_field() {
    let schema = load_test_schema();
    let rating = schema.get_component("rating_display").unwrap();
    assert_eq!(rating.get_field_type("rating"), Some("f64"));
    assert_eq!(rating.get_field_type("review_count"), Some("i64"));
}

#[test]
fn test_get_component_schema_alias() {
    let schema = load_test_schema();
    let product_alias = schema.get_component_schema("product_card").unwrap();
    let product_orig = schema.get_component("product_card").unwrap();

    assert_eq!(product_alias.description, product_orig.description);
}

#[test]
fn test_load_actual_config() {
    // Test loading the actual config file
    let toml_content = include_str!("../../config/ui.toml");
    let config: UiSchemaConfig = toml::from_str(toml_content).expect("Failed to parse ui.toml");

    assert!(config.has_component("post_card"));
    assert!(config.has_component("text"));
    assert!(config.has_component("container"));
}
