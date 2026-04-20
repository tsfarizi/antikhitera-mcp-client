#[test]
fn test_menu_next_navigation() {
    let items = vec![
        MenuItem::new("Item 1"),
        MenuItem::new("Item 2"),
        MenuItem::new("Item 3"),
    ];
    let mut menu = Menu::new("Test", items);

    assert_eq!(menu.selected_index(), Some(0));

    menu.next();
    assert_eq!(menu.selected_index(), Some(1));

    menu.next();
    assert_eq!(menu.selected_index(), Some(2));

    // Wrap around to first
    menu.next();
    assert_eq!(menu.selected_index(), Some(0));
}

