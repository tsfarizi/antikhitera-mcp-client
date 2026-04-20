#[tokio::test]
async fn guardrail_chain_preserves_order_and_rate_limit_rejects_second_dispatch() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let guardrails = GuardrailChain::new()
        .with_guardrail(Arc::new(RateLimitGuardrail::new(1, 60_000)))
        .with_guardrail(Arc::new(AlwaysRejectGuardrail));
    let orchestrator = build_orchestrator(call_count.clone()).with_guardrails(guardrails);

    let first = orchestrator.dispatch(AgentTask::new("task one")).await;
    let second = orchestrator.dispatch(AgentTask::new("task two")).await;

    assert!(!first.success);
    assert_eq!(
        first.metadata.guardrail_name.as_deref(),
        Some("always_reject")
    );

    assert!(!second.success);
    assert_eq!(second.error_kind, Some(ErrorKind::Transient));
    assert_eq!(
        second.metadata.guardrail_name.as_deref(),
        Some("rate_limit")
    );
    assert_eq!(
        second.metadata.guardrail_stage.as_deref(),
        Some("pre_check")
    );
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}

