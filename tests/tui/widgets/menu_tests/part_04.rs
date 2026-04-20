#[test]
fn test_menu_creation_with_items() {
    let items = vec![
        MenuItem::new("Item 1"),
        MenuItem::new("Item 2"),
        MenuItem::new("Item 3"),
    ];
    let menu = Menu::new("Test Menu", items);

    assert_eq!(menu.title, "Test Menu");
    assert_eq!(menu.items.len(), 3);
    assert_eq!(menu.selected_index(), Some(0));
}

