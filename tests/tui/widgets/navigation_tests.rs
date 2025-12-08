//! Menu navigation tests

use antikhitera_mcp_client::tui::widgets::{Menu, MenuItem};

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

#[test]
fn test_menu_select_out_of_bounds() {
    let items = vec![MenuItem::new("Item 1")];
    let mut menu = Menu::new("Test", items);

    menu.select(100); // Out of bounds
    assert_eq!(menu.selected_index(), Some(0)); // Should remain unchanged
}

#[test]
fn test_menu_navigation_empty() {
    let mut menu = Menu::new("Empty", vec![]);

    menu.next(); // Should not panic
    menu.previous(); // Should not panic

    assert_eq!(menu.selected_index(), None);
}
