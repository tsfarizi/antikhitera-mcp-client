#[test]
fn test_menu_select_out_of_bounds() {
    let items = vec![MenuItem::new("Item 1")];
    let mut menu = Menu::new("Test", items);

    menu.select(100); // Out of bounds
    assert_eq!(menu.selected_index(), Some(0)); // Should remain unchanged
}

