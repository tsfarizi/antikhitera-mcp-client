#[tokio::test]
async fn timeout_guardrail_blocks_dispatch_before_provider_is_called() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let orchestrator = build_orchestrator(call_count.clone())
        .with_guardrail(Arc::new(TimeoutGuardrail::new(1_000).require_timeout()));

    let result = orchestrator
        .dispatch(AgentTask::new("review this code"))
        .await;

    assert!(!result.success);
    assert_eq!(result.error_kind, Some(ErrorKind::Permanent));
    assert_eq!(result.metadata.guardrail_name.as_deref(), Some("timeout"));
    assert_eq!(
        result.metadata.guardrail_stage.as_deref(),
        Some("pre_check")
    );
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}

