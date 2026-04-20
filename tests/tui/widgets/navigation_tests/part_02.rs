#[test]
fn test_menu_previous_navigation() {
    let items = vec![
        MenuItem::new("Item 1"),
        MenuItem::new("Item 2"),
        MenuItem::new("Item 3"),
    ];
    let mut menu = Menu::new("Test", items);

    assert_eq!(menu.selected_index(), Some(0));

    // Wrap around to last
    menu.previous();
    assert_eq!(menu.selected_index(), Some(2));

    menu.previous();
    assert_eq!(menu.selected_index(), Some(1));

    menu.previous();
    assert_eq!(menu.selected_index(), Some(0));
}

