#[test]
fn test_menu_select() {
    let items = vec![
        MenuItem::new("Item 1"),
        MenuItem::new("Item 2"),
        MenuItem::new("Item 3"),
    ];
    let mut menu = Menu::new("Test", items);

    menu.select(2);
    assert_eq!(menu.selected_index(), Some(2));

    menu.select(0);
    assert_eq!(menu.selected_index(), Some(0));
}

