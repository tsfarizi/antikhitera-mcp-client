#[test]
fn test_nav_action_up() {
    let action = NavAction::from(make_key(KeyCode::Up));
    assert_eq!(action, NavAction::Up);

    let action = NavAction::from(make_key(KeyCode::Char('k')));
    assert_eq!(action, NavAction::Up);
}

