#[test]
fn test_nav_action_back() {
    let action = NavAction::from(make_key(KeyCode::Esc));
    assert_eq!(action, NavAction::Back);

    let action = NavAction::from(make_key(KeyCode::Backspace));
    assert_eq!(action, NavAction::Back);
}

