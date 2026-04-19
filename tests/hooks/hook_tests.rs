use std::collections::HashMap;
use std::sync::Arc;

use antikythera_core::{
    AuthHook, CallerContext, CorrelationHook, HookContext, HookError, HookOperation, HookRegistry,
    HostHookMiddleware, InMemoryTelemetryHook, PolicyDecision, PolicyDecisionHook, PolicyTarget,
    TelemetryEvent,
};

struct RequireUser;

impl AuthHook for RequireUser {
    fn name(&self) -> &'static str {
        "require_user"
    }

    fn authorize(&self, context: &HookContext) -> Result<(), HookError> {
        if context.caller.user_id.is_some() {
            Ok(())
        } else {
            Err(HookError::new(self.name(), "missing user"))
        }
    }
}

struct AddCorrelation;

impl CorrelationHook for AddCorrelation {
    fn name(&self) -> &'static str {
        "add_correlation"
    }

    fn apply(&self, context: &mut HookContext) -> Result<(), HookError> {
        if context.correlation_id.is_none() {
            context.correlation_id = Some("corr-hook".to_string());
            context.caller.correlation_id = Some("corr-hook".to_string());
        }
        context
            .metadata
            .insert("trace_origin".to_string(), "integration-test".to_string());
        Ok(())
    }
}

struct ToolPolicy;

impl PolicyDecisionHook for ToolPolicy {
    fn name(&self) -> &'static str {
        "tool_policy"
    }

    fn decide(
        &self,
        input: &antikythera_core::PolicyDecisionInput,
    ) -> Result<PolicyDecision, HookError> {
        match &input.target {
            PolicyTarget::Tool { tool_name } if tool_name == "blocked" => Ok(PolicyDecision::Deny),
            _ => Ok(PolicyDecision::Allow),
        }
    }
}

#[test]
fn middleware_prepare_context_updates_correlation_and_metadata() {
    let middleware = HostHookMiddleware::new(
        HookRegistry::new()
            .with_auth_hook(Arc::new(RequireUser))
            .with_correlation_hook(Arc::new(AddCorrelation)),
    );

    let prepared = middleware
        .prepare_context(
            HookContext::new(
                CallerContext::new().with_user_id("user-a"),
                HookOperation::AgentRun,
            )
            .with_session_id("sess-a"),
        )
        .expect("prepare context should succeed");

    assert_eq!(prepared.correlation_id.as_deref(), Some("corr-hook"));
    assert_eq!(
        prepared.metadata.get("trace_origin").map(String::as_str),
        Some("integration-test")
    );
}

#[test]
fn middleware_policy_denies_blocked_tool() {
    let middleware =
        HostHookMiddleware::new(HookRegistry::new().with_policy_hook(Arc::new(ToolPolicy)));
    let context = HookContext::new(
        CallerContext::new().with_user_id("user-a"),
        HookOperation::ToolCall,
    );

    let decision = middleware
        .authorize_tool(&context, "blocked")
        .expect("policy evaluation should succeed");

    assert_eq!(decision, PolicyDecision::Deny);
}

#[test]
fn telemetry_hook_records_external_event() {
    let sink = InMemoryTelemetryHook::new();
    let middleware =
        HostHookMiddleware::new(HookRegistry::new().with_telemetry_hook(Arc::new(sink.clone())));
    let context = HookContext::new(
        CallerContext::new().with_user_id("user-a"),
        HookOperation::AgentRun,
    )
    .with_correlation_id("corr-a")
    .with_session_id("sess-a");

    let event = TelemetryEvent {
        event_type: "agent_finished".to_string(),
        correlation_id: context.correlation_id.clone(),
        session_id: context.session_id.clone(),
        timestamp_ms: 1,
        attributes: HashMap::new(),
    };

    middleware
        .emit_event(&context, event)
        .expect("emit event should succeed");

    assert_eq!(sink.snapshot().len(), 1);
    assert_eq!(sink.snapshot()[0].event_type, "agent_finished");
}
