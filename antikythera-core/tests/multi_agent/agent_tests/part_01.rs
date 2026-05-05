use antikythera_core::application::agent::AgentStateSnapshot;

#[test]
fn test_postcard_serialization() {
    let state = AgentStateSnapshot::new("test".into(), "agent".into());

    // Serialize
    let bytes = state.to_postcard().unwrap();

    // Deserialize
    let loaded = AgentStateSnapshot::from_postcard(&bytes).unwrap();

    assert_eq!(loaded.context_id, state.context_id);
    assert_eq!(loaded.agent_id, state.agent_id);
    assert_eq!(loaded.schema_version, state.schema_version);
}
