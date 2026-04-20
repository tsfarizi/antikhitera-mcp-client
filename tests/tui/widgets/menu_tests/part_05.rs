#[test]
fn test_menu_with_subtitle() {
    let items = vec![MenuItem::new("Item")];
    let menu = Menu::new("Title", items).with_subtitle("Subtitle");

    assert_eq!(menu.subtitle, Some("Subtitle".to_string()));
}
