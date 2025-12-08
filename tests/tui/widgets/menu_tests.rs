//! Menu widget tests

use antikhitera_mcp_client::tui::widgets::{Menu, MenuItem};

#[test]
fn test_menu_item_creation() {
    let item = MenuItem::new("Test Item");
    assert_eq!(item.label, "Test Item");
    assert!(!item.is_default);
}

#[test]
fn test_menu_item_with_default_marker() {
    let item = MenuItem::new("Default Item").with_default_marker(true);
    assert_eq!(item.label, "Default Item");
    assert!(item.is_default);
}

#[test]
fn test_menu_creation_empty() {
    let menu = Menu::new("Empty Menu", vec![]);
    assert_eq!(menu.title, "Empty Menu");
    assert!(menu.items.is_empty());
    assert_eq!(menu.selected_index(), None);
}

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

#[test]
fn test_menu_with_subtitle() {
    let items = vec![MenuItem::new("Item")];
    let menu = Menu::new("Title", items).with_subtitle("Subtitle");

    assert_eq!(menu.subtitle, Some("Subtitle".to_string()));
}
