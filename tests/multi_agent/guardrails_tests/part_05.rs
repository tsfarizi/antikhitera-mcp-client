#[tokio::test]
async fn cancellation_guardrail_blocks_dispatch_after_orchestrator_cancel() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let orchestrator = build_orchestrator(call_count.clone())
        .with_guardrail(Arc::new(CancellationGuardrail::new()));

    orchestrator.cancel();
    let result = orchestrator
        .dispatch(AgentTask::new("summarize logs"))
        .await;

    assert!(!result.success);
    assert_eq!(result.error_kind, Some(ErrorKind::Cancelled));
    assert!(result.metadata.cancelled);
    assert_eq!(
        result.metadata.guardrail_name.as_deref(),
        Some("cancellation")
    );
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}
