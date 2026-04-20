#[test]
fn test_nav_action_force_quit() {
    let action = NavAction::from(make_key(KeyCode::Char('q')));
    assert_eq!(action, NavAction::ForceQuit);

    let action = NavAction::from(make_key_with_ctrl(KeyCode::Char('q')));
    assert_eq!(action, NavAction::ForceQuit);
}


#[test]
fn test_nav_action_none() {
    let action = NavAction::from(make_key(KeyCode::Char('x')));
    assert_eq!(action, NavAction::None);

    let action = NavAction::from(make_key(KeyCode::F(1)));
    assert_eq!(action, NavAction::None);
}
