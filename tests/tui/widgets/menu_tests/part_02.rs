#[test]
fn test_menu_item_with_default_marker() {
    let item = MenuItem::new("Default Item").with_default_marker(true);
    assert_eq!(item.label, "Default Item");
    assert!(item.is_default);
}

