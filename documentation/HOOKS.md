# Host Integration Hooks

## Overview

The host integration hooks module provides an optional middleware layer for
embedding hosts that need to attach identity, correlation metadata, access
policy checks, and structured telemetry around framework operations.

Implementation lives in `antikythera_core::application::hooks` and is designed
to work alongside the existing observability primitives in
`antikythera_core::application::observability`.

## Core Concepts

### HookContext

`HookContext` carries the mutable host-facing request state:

- caller identity (`CallerContext`)
- operation kind (`HookOperation`)
- optional session id
- correlation id
- free-form metadata

### Hook Types

- `AuthHook`
  Validates caller identity and permission propagation.

- `CorrelationHook`
  Mutates correlation id and request metadata before execution continues.

- `PolicyDecisionHook`
  Evaluates model or tool access requests and returns `Allow`, `Deny`, or `Audit`.

- `TelemetryHook`
  Receives structured `TelemetryEvent` values for audit or observability export.

### HookRegistry

`HookRegistry` is a typed registry for all hook kinds. It supports explicit
registration and builder-style composition.

### HostHookMiddleware

`HostHookMiddleware` is the host-facing execution facade. It provides:

- `prepare_context(...)`
- `authorize_model(...)`
- `authorize_tool(...)`
- `emit_event(...)`

## Quick Start

```rust
use std::sync::Arc;

use antikythera_core::{
    AuthHook, CallerContext, HookContext, HookError, HookOperation, HookRegistry,
    HostHookMiddleware, PolicyDecision, PolicyDecisionHook, PolicyTarget,
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
            Err(HookError::new(self.name(), "user_id is required"))
        }
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
            PolicyTarget::Tool { tool_name } if tool_name == "delete-all" => Ok(PolicyDecision::Deny),
            _ => Ok(PolicyDecision::Allow),
        }
    }
}

let middleware = HostHookMiddleware::new(
    HookRegistry::new()
        .with_auth_hook(Arc::new(RequireUser))
        .with_policy_hook(Arc::new(ToolPolicy)),
);

let context = middleware.prepare_context(
    HookContext::new(
        CallerContext::new().with_user_id("user-123"),
        HookOperation::ToolCall,
    )
    .with_session_id("sess-42"),
)?;

let decision = middleware.authorize_tool(&context, "search")?;
assert_eq!(decision, PolicyDecision::Allow);
# Ok::<(), HookError>(())
```

## Policy Flow

1. Host builds a `HookContext`
2. `prepare_context` runs auth hooks, then correlation hooks
3. Host requests access checks via `authorize_model` or `authorize_tool`
4. Host emits audit or lifecycle events through `emit_event`

If no hooks are registered, default behavior is permissive and no-op.

## Built-In Utilities

### InMemoryTelemetryHook

Useful for tests and local audit snapshots.

```rust
use std::sync::Arc;

use antikythera_core::{
    CallerContext, HookContext, HookOperation, HookRegistry, HostHookMiddleware,
    InMemoryTelemetryHook, TelemetryEvent,
};

let sink = InMemoryTelemetryHook::new();
let middleware = HostHookMiddleware::new(
    HookRegistry::new().with_telemetry_hook(Arc::new(sink.clone())),
);
let context = HookContext::new(CallerContext::new(), HookOperation::AgentRun)
    .with_correlation_id("corr-1")
    .with_session_id("sess-1");

middleware.emit_event(
    &context,
    TelemetryEvent::new("agent_start", context.correlation_id.clone(), context.session_id.clone()),
)?;

assert_eq!(sink.snapshot().len(), 1);
# Ok::<(), antikythera_core::HookError>(())
```

## Design Notes

- Hooks are optional. No hooks means no extra host middleware work.
- Auth hooks fail fast.
- Correlation hooks run after auth so metadata can be enriched only for valid callers.
- Policy hooks aggregate to:
  - `Deny` if any hook denies
  - `Audit` if at least one hook audits and none deny
  - `Allow` otherwise
- Telemetry hooks fan out to every registered sink.

## Testing

Coverage includes:

- 13 unit tests in `application/hooks.rs`
- 3 external integration tests in `tests/hooks/hook_tests.rs`
- serialization round-trips for policy and context types
- auth failure propagation and middleware ordering

## Backward Compatibility

The hooks layer is additive.

- Existing client and agent APIs remain unchanged.
- Hosts can adopt hooks incrementally.
- Existing observability primitives remain valid and reusable.