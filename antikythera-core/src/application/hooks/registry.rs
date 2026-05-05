use std::sync::Arc;

use super::types::{
    AuthHook, CorrelationHook, HookContext, HookError, PolicyDecision, PolicyDecisionHook,
    PolicyTarget, TelemetryHook,
};
use crate::application::observability::TelemetryEvent;
use crate::logging::AgentLogger;

/// Registry of all host integration hooks.
#[derive(Clone, Default)]
pub struct HookRegistry {
    pub(super) auth_hooks: Vec<Arc<dyn AuthHook>>,
    pub(super) correlation_hooks: Vec<Arc<dyn CorrelationHook>>,
    pub(super) policy_hooks: Vec<Arc<dyn PolicyDecisionHook>>,
    pub(super) telemetry_hooks: Vec<Arc<dyn TelemetryHook>>,
    log: Option<AgentLogger>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a logger for registration and execution tracing.
    pub fn with_logger(mut self, log: AgentLogger) -> Self {
        self.log = Some(log);
        self
    }

    pub fn register_auth_hook(&mut self, hook: Arc<dyn AuthHook>) {
        if let Some(ref log) = self.log {
            log.info("Auth hook registered");
        }
        self.auth_hooks.push(hook);
    }

    pub fn register_correlation_hook(&mut self, hook: Arc<dyn CorrelationHook>) {
        if let Some(ref log) = self.log {
            log.info("Correlation hook registered");
        }
        self.correlation_hooks.push(hook);
    }

    pub fn register_policy_hook(&mut self, hook: Arc<dyn PolicyDecisionHook>) {
        if let Some(ref log) = self.log {
            log.info("Policy decision hook registered");
        }
        self.policy_hooks.push(hook);
    }

    pub fn register_telemetry_hook(&mut self, hook: Arc<dyn TelemetryHook>) {
        if let Some(ref log) = self.log {
            log.info("Telemetry hook registered");
        }
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
    log: Option<AgentLogger>,
}

impl HostHookMiddleware {
    /// Create middleware from a registry.
    pub fn new(registry: HookRegistry) -> Self {
        let log = registry.log.clone();
        Self { registry, log }
    }

    /// Borrow the underlying registry.
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    /// Run auth and correlation hooks and return the resulting context.
    pub fn prepare_context(&self, mut context: HookContext) -> Result<HookContext, HookError> {
        if let Some(ref log) = self.log {
            log.debug("Preparing hook context");
        }
        for hook in &self.registry.auth_hooks {
            hook.authorize(&context)?;
        }
        for hook in &self.registry.correlation_hooks {
            hook.apply(&mut context)?;
        }
        if let Some(ref log) = self.log {
            log.debug("Hook context prepared");
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
                PolicyDecision::Deny => {
                    if let Some(ref log) = self.log {
                        log.warn("Policy decision: deny");
                    }
                    return Ok(PolicyDecision::Deny);
                }
            }
        }

        let decision = if saw_audit {
            if let Some(ref log) = self.log {
                log.info("Policy decision: audit");
            }
            PolicyDecision::Audit
        } else {
            if let Some(ref log) = self.log {
                log.debug("Policy decision: allow");
            }
            PolicyDecision::Allow
        };
        Ok(decision)
    }

    /// Emit a telemetry event to all registered telemetry hooks.
    pub fn emit_event(
        &self,
        context: &HookContext,
        event: TelemetryEvent,
    ) -> Result<(), HookError> {
        if let Some(ref log) = self.log {
            log.debug("Emitting telemetry event");
        }
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


