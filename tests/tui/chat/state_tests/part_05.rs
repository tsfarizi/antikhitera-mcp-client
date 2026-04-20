#[test]
fn test_toggle_agent_mode() {
    let mut state = ChatState::new();
    assert!(state.agent_mode);

    state.toggle_agent_mode();
    assert!(!state.agent_mode);

    state.toggle_agent_mode();
    assert!(state.agent_mode);
}


#[test]
fn test_loading_tick() {
    let mut state = ChatState::new();
    state.loading = true;
    state.loading_frame = 0;

    state.tick_loading();
    assert_eq!(state.loading_frame, 1);

    state.loading_frame = 3;
    state.tick_loading();
    assert_eq!(state.loading_frame, 0);
}
