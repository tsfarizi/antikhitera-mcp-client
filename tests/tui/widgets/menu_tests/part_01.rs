#[test]
fn test_menu_item_creation() {
    let item = MenuItem::new("Test Item");
    assert_eq!(item.label, "Test Item");
    assert!(!item.is_default);
}

