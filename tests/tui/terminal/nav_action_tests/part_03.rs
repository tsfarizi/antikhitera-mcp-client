#[test]
fn test_nav_action_select() {
    let action = NavAction::from(make_key(KeyCode::Enter));
    assert_eq!(action, NavAction::Select);

    let action = NavAction::from(make_key(KeyCode::Char(' ')));
    assert_eq!(action, NavAction::Select);
}

