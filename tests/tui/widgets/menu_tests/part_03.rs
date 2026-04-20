#[test]
fn test_menu_creation_empty() {
    let menu = Menu::new("Empty Menu", vec![]);
    assert_eq!(menu.title, "Empty Menu");
    assert!(menu.items.is_empty());
    assert_eq!(menu.selected_index(), None);
}

