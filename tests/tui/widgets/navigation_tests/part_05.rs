#[test]
fn test_menu_navigation_empty() {
    let mut menu = Menu::new("Empty", vec![]);

    menu.next(); // Should not panic
    menu.previous(); // Should not panic

    assert_eq!(menu.selected_index(), None);
}
