//! Host integration hooks.
//!
//! This module provides an optional middleware layer for hosts that need to
//! propagate caller identity, correlation metadata, access policy decisions,
//! and structured telemetry around framework operations.
//!
//! The hooks are intentionally host-owned and opt-in. If no hooks are
//! registered, the framework can continue to run with zero extra coordination.

pub mod registry;
pub mod types;

pub use registry::{HookRegistry, HostHookMiddleware, InMemoryTelemetryHook};
pub use types::{
    AuthHook, CorrelationHook, HookContext, HookError, HookOperation, PolicyDecision,
    PolicyDecisionHook, PolicyDecisionInput, PolicyTarget, TelemetryHook,
};
