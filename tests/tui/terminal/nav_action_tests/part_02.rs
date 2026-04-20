#[test]
fn test_nav_action_down() {
    let action = NavAction::from(make_key(KeyCode::Down));
    assert_eq!(action, NavAction::Down);

    let action = NavAction::from(make_key(KeyCode::Char('j')));
    assert_eq!(action, NavAction::Down);
}

