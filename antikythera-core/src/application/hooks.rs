//! Host integration hooks.
//!
//! This module provides an optional middleware layer for hosts that need to
//! propagate caller identity, correlation metadata, access policy decisions,
//! and structured telemetry around framework operations.
//!
//! The hooks are intentionally host-owned and opt-in. If no hooks are
//! registered, the framework can continue to run with zero extra coordination.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::observability::{CallerContext, TelemetryEvent};

/// Operation being processed by the hook middleware.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOperation {
    AgentRun,
    ToolCall,
    ModelRequest,
    SessionRead,
    SessionWrite,
    Custom,
}

/// Access policy target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyTarget {
    Model { provider: String, model: String },
    Tool { tool_name: String },
}

/// Input sent to policy hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecisionInput {
    pub caller: CallerContext,
    pub session_id: Option<String>,
    pub correlation_id: Option<String>,
    pub target: PolicyTarget,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Result returned by policy hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Deny,
    Audit,
}

/// Hook middleware error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookError {
    pub hook_name: String,
    pub message: String,
}

impl HookError {
    /// Create a new hook error.
    pub fn new(hook_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            hook_name: hook_name.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.hook_name, self.message)
    }
}

impl std::error::Error for HookError {}

/// Mutable request context passed through middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub caller: CallerContext,
    pub operation: HookOperation,
    pub session_id: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl HookContext {
    /// Create a new hook context.
    pub fn new(caller: CallerContext, operation: HookOperation) -> Self {
        Self {
            correlation_id: caller.correlation_id.clone(),
            caller,
            operation,
            session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Attach a session identifier.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Override or set correlation id.
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        let correlation_id = correlation_id.into();
        self.correlation_id = Some(correlation_id.clone());
        self.caller.correlation_id = Some(correlation_id);
        self
    }

    /// Add metadata to the request context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build policy input for a specific target.
    pub fn policy_input(&self, target: PolicyTarget) -> PolicyDecisionInput {
        PolicyDecisionInput {
            caller: self.caller.clone(),
            session_id: self.session_id.clone(),
            correlation_id: self.correlation_id.clone(),
            target,
            metadata: self.metadata.clone(),
        }
    }
}

/// Hook for caller identity and permission propagation.
pub trait AuthHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn authorize(&self, context: &HookContext) -> Result<(), HookError>;
}

/// Hook for correlation and request metadata mutation.
pub trait CorrelationHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, context: &mut HookContext) -> Result<(), HookError>;
}

/// Hook for model or tool policy decisions.
pub trait PolicyDecisionHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn decide(&self, input: &PolicyDecisionInput) -> Result<PolicyDecision, HookError>;
}

/// Hook for structured telemetry and audit emission.
pub trait TelemetryHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn emit(&self, context: &HookContext, event: &TelemetryEvent) -> Result<(), HookError>;
}

/// Registry of all host integration hooks.
#[derive(Clone, Default)]
pub struct HookRegistry {
    auth_hooks: Vec<Arc<dyn AuthHook>>,
    correlation_hooks: Vec<Arc<dyn CorrelationHook>>,
    policy_hooks: Vec<Arc<dyn PolicyDecisionHook>>,
    telemetry_hooks: Vec<Arc<dyn TelemetryHook>>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_auth_hook(&mut self, hook: Arc<dyn AuthHook>) {
        self.auth_hooks.push(hook);
    }

    pub fn register_correlation_hook(&mut self, hook: Arc<dyn CorrelationHook>) {
        self.correlation_hooks.push(hook);
    }

    pub fn register_policy_hook(&mut self, hook: Arc<dyn PolicyDecisionHook>) {
        self.policy_hooks.push(hook);
    }

    pub fn register_telemetry_hook(&mut self, hook: Arc<dyn TelemetryHook>) {
        self.telemetry_hooks.push(hook);
    }

    pub fn with_auth_hook(mut self, hook: Arc<dyn AuthHook>) -> Self {
        self.register_auth_hook(hook);
        self
    }

    pub fn with_correlation_hook(mut self, hook: Arc<dyn CorrelationHook>) -> Self {
        self.register_correlation_hook(hook);
        self
    }

    pub fn with_policy_hook(mut self, hook: Arc<dyn PolicyDecisionHook>) -> Self {
        self.register_policy_hook(hook);
        self
    }

    pub fn with_telemetry_hook(mut self, hook: Arc<dyn TelemetryHook>) -> Self {
        self.register_telemetry_hook(hook);
        self
    }

    pub fn auth_hook_count(&self) -> usize {
        self.auth_hooks.len()
    }

    pub fn correlation_hook_count(&self) -> usize {
        self.correlation_hooks.len()
    }

    pub fn policy_hook_count(&self) -> usize {
        self.policy_hooks.len()
    }

    pub fn telemetry_hook_count(&self) -> usize {
        self.telemetry_hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.auth_hooks.is_empty()
            && self.correlation_hooks.is_empty()
            && self.policy_hooks.is_empty()
            && self.telemetry_hooks.is_empty()
    }
}

/// Middleware facade for host integration hooks.
#[derive(Clone, Default)]
pub struct HostHookMiddleware {
    registry: HookRegistry,
}

impl HostHookMiddleware {
    /// Create middleware from a registry.
    pub fn new(registry: HookRegistry) -> Self {
        Self { registry }
    }

    /// Borrow the underlying registry.
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    /// Run auth and correlation hooks and return the resulting context.
    pub fn prepare_context(&self, mut context: HookContext) -> Result<HookContext, HookError> {
        for hook in &self.registry.auth_hooks {
            hook.authorize(&context)?;
        }
        for hook in &self.registry.correlation_hooks {
            hook.apply(&mut context)?;
        }
        Ok(context)
    }

    /// Evaluate model access against all registered policy hooks.
    pub fn authorize_model(
        &self,
        context: &HookContext,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<PolicyDecision, HookError> {
        self.evaluate_policy(
            context,
            PolicyTarget::Model {
                provider: provider.into(),
                model: model.into(),
            },
        )
    }

    /// Evaluate tool access against all registered policy hooks.
    pub fn authorize_tool(
        &self,
        context: &HookContext,
        tool_name: impl Into<String>,
    ) -> Result<PolicyDecision, HookError> {
        self.evaluate_policy(
            context,
            PolicyTarget::Tool {
                tool_name: tool_name.into(),
            },
        )
    }

    fn evaluate_policy(
        &self,
        context: &HookContext,
        target: PolicyTarget,
    ) -> Result<PolicyDecision, HookError> {
        let input = context.policy_input(target);
        let mut saw_audit = false;

        for hook in &self.registry.policy_hooks {
            match hook.decide(&input)? {
                PolicyDecision::Allow => {}
                PolicyDecision::Audit => saw_audit = true,
                PolicyDecision::Deny => return Ok(PolicyDecision::Deny),
            }
        }

        if saw_audit {
            Ok(PolicyDecision::Audit)
        } else {
            Ok(PolicyDecision::Allow)
        }
    }

    /// Emit a telemetry event to all registered telemetry hooks.
    pub fn emit_event(
        &self,
        context: &HookContext,
        event: TelemetryEvent,
    ) -> Result<(), HookError> {
        for hook in &self.registry.telemetry_hooks {
            hook.emit(context, &event)?;
        }
        Ok(())
    }
}

/// In-memory telemetry sink suitable for tests and host-side audit snapshots.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTelemetryHook {
    events: Arc<std::sync::Mutex<Vec<TelemetryEvent>>>,
}

impl InMemoryTelemetryHook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl TelemetryHook for InMemoryTelemetryHook {
    fn name(&self) -> &'static str {
        "in_memory_telemetry"
    }

    fn emit(&self, _context: &HookContext, event: &TelemetryEvent) -> Result<(), HookError> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct AllowAuth;

    impl AuthHook for AllowAuth {
        fn name(&self) -> &'static str {
            "allow_auth"
        }

        fn authorize(&self, _context: &HookContext) -> Result<(), HookError> {
            Ok(())
        }
    }

    struct DenyMissingUser;

    impl AuthHook for DenyMissingUser {
        fn name(&self) -> &'static str {
            "deny_missing_user"
        }

        fn authorize(&self, context: &HookContext) -> Result<(), HookError> {
            if context.caller.user_id.is_some() {
                Ok(())
            } else {
                Err(HookError::new(self.name(), "user_id is required"))
            }
        }
    }

    struct InjectCorrelation;

    impl CorrelationHook for InjectCorrelation {
        fn name(&self) -> &'static str {
            "inject_correlation"
        }

        fn apply(&self, context: &mut HookContext) -> Result<(), HookError> {
            if context.correlation_id.is_none() {
                *context = context.clone().with_correlation_id("corr-generated");
            }
            context
                .metadata
                .insert("source".to_string(), "hook".to_string());
            Ok(())
        }
    }

    struct DenyDangerousTool;

    impl PolicyDecisionHook for DenyDangerousTool {
        fn name(&self) -> &'static str {
            "deny_dangerous_tool"
        }

        fn decide(&self, input: &PolicyDecisionInput) -> Result<PolicyDecision, HookError> {
            match &input.target {
                PolicyTarget::Tool { tool_name } if tool_name == "delete-all" => {
                    Ok(PolicyDecision::Deny)
                }
                _ => Ok(PolicyDecision::Allow),
            }
        }
    }

    struct AuditAllModels;

    impl PolicyDecisionHook for AuditAllModels {
        fn name(&self) -> &'static str {
            "audit_all_models"
        }

        fn decide(&self, input: &PolicyDecisionInput) -> Result<PolicyDecision, HookError> {
            match input.target {
                PolicyTarget::Model { .. } => Ok(PolicyDecision::Audit),
                PolicyTarget::Tool { .. } => Ok(PolicyDecision::Allow),
            }
        }
    }

    fn caller() -> CallerContext {
        CallerContext::new()
            .with_user_id("user-1")
            .with_source("test")
    }

    #[test]
    fn hook_context_builder_sets_fields() {
        let context = HookContext::new(caller(), HookOperation::AgentRun)
            .with_session_id("sess-1")
            .with_correlation_id("corr-1")
            .with_metadata("env", "test");

        assert_eq!(context.session_id.as_deref(), Some("sess-1"));
        assert_eq!(context.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(
            context.metadata.get("env").map(String::as_str),
            Some("test")
        );
    }

    #[test]
    fn policy_input_copies_context_state() {
        let context = HookContext::new(caller(), HookOperation::ToolCall)
            .with_session_id("sess-2")
            .with_correlation_id("corr-2")
            .with_metadata("scope", "internal");
        let input = context.policy_input(PolicyTarget::Tool {
            tool_name: "search".to_string(),
        });

        assert_eq!(input.session_id.as_deref(), Some("sess-2"));
        assert_eq!(input.correlation_id.as_deref(), Some("corr-2"));
        assert_eq!(
            input.metadata.get("scope").map(String::as_str),
            Some("internal")
        );
    }

    #[test]
    fn hook_registry_counts_registered_hooks() {
        let registry = HookRegistry::new()
            .with_auth_hook(Arc::new(AllowAuth))
            .with_correlation_hook(Arc::new(InjectCorrelation))
            .with_policy_hook(Arc::new(DenyDangerousTool))
            .with_telemetry_hook(Arc::new(InMemoryTelemetryHook::new()));

        assert_eq!(registry.auth_hook_count(), 1);
        assert_eq!(registry.correlation_hook_count(), 1);
        assert_eq!(registry.policy_hook_count(), 1);
        assert_eq!(registry.telemetry_hook_count(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn hook_registry_default_is_empty() {
        assert!(HookRegistry::new().is_empty());
    }

    #[test]
    fn middleware_prepare_context_runs_auth_and_correlation_hooks() {
        let middleware = HostHookMiddleware::new(
            HookRegistry::new()
                .with_auth_hook(Arc::new(AllowAuth))
                .with_correlation_hook(Arc::new(InjectCorrelation)),
        );

        let prepared = middleware
            .prepare_context(HookContext::new(caller(), HookOperation::AgentRun))
            .expect("prepare_context should succeed");

        assert_eq!(prepared.correlation_id.as_deref(), Some("corr-generated"));
        assert_eq!(
            prepared.metadata.get("source").map(String::as_str),
            Some("hook")
        );
    }

    #[test]
    fn middleware_prepare_context_propagates_auth_error() {
        let middleware =
            HostHookMiddleware::new(HookRegistry::new().with_auth_hook(Arc::new(DenyMissingUser)));

        let error = middleware
            .prepare_context(HookContext::new(
                CallerContext::new(),
                HookOperation::AgentRun,
            ))
            .expect_err("missing user should be rejected");

        assert_eq!(error.hook_name, "deny_missing_user");
    }

    #[test]
    fn authorize_tool_returns_deny_when_any_policy_denies() {
        let middleware = HostHookMiddleware::new(
            HookRegistry::new().with_policy_hook(Arc::new(DenyDangerousTool)),
        );
        let context = HookContext::new(caller(), HookOperation::ToolCall);

        let decision = middleware
            .authorize_tool(&context, "delete-all")
            .expect("policy decision should succeed");

        assert_eq!(decision, PolicyDecision::Deny);
    }

    #[test]
    fn authorize_model_returns_audit_when_policy_requests_it() {
        let middleware =
            HostHookMiddleware::new(HookRegistry::new().with_policy_hook(Arc::new(AuditAllModels)));
        let context = HookContext::new(caller(), HookOperation::ModelRequest);

        let decision = middleware
            .authorize_model(&context, "host", "gpt-host")
            .expect("policy decision should succeed");

        assert_eq!(decision, PolicyDecision::Audit);
    }

    #[test]
    fn authorize_model_defaults_to_allow_when_no_policy_hooks_exist() {
        let middleware = HostHookMiddleware::new(HookRegistry::new());
        let decision = middleware
            .authorize_model(
                &HookContext::new(caller(), HookOperation::ModelRequest),
                "host",
                "gpt",
            )
            .expect("no policy hooks should allow");
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn in_memory_telemetry_hook_records_events() {
        let hook = InMemoryTelemetryHook::new();
        let middleware = HostHookMiddleware::new(
            HookRegistry::new().with_telemetry_hook(Arc::new(hook.clone())),
        );
        let context = HookContext::new(caller(), HookOperation::AgentRun)
            .with_correlation_id("corr-telemetry")
            .with_session_id("sess-telemetry");
        let event = TelemetryEvent::new(
            "agent_start",
            context.correlation_id.clone(),
            context.session_id.clone(),
        );

        middleware
            .emit_event(&context, event)
            .expect("telemetry emit should succeed");

        assert_eq!(hook.snapshot().len(), 1);
        assert_eq!(hook.snapshot()[0].event_type, "agent_start");
    }

    #[test]
    fn hook_error_display_includes_name_and_message() {
        let error = HookError::new("auth", "denied");
        assert_eq!(error.to_string(), "auth: denied");
    }

    #[test]
    fn policy_target_serialization_roundtrip() {
        let target = PolicyTarget::Model {
            provider: "host".to_string(),
            model: "gpt-host".to_string(),
        };
        let json = serde_json::to_string(&target).expect("serialize");
        let restored: PolicyTarget = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, target);
    }

    #[test]
    fn policy_decision_input_serialization_roundtrip() {
        let input = HookContext::new(caller(), HookOperation::ToolCall)
            .with_correlation_id("corr-3")
            .policy_input(PolicyTarget::Tool {
                tool_name: "search".to_string(),
            });

        let json = serde_json::to_string(&input).expect("serialize");
        let restored: PolicyDecisionInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.correlation_id.as_deref(), Some("corr-3"));
    }
}
