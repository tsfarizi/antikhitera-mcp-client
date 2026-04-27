use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::application::observability::{CallerContext, TelemetryEvent};

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
