# Multi-Agent Guardrails

## Overview

The multi-agent guardrails module adds composable execution policy checks to the
orchestrator without changing the default runtime behavior. Guardrails are
strictly opt-in. If no guardrails are configured, task dispatch behaves exactly
as it did before.

The implementation lives in `antikythera_core::application::agent::multi_agent::guardrails`
and integrates directly with `MultiAgentOrchestrator`.

## What It Adds

- `TaskGuardrail` trait for custom policies
- `GuardrailChain` for ordered composition
- `TimeoutGuardrail` for timeout policy validation
- `BudgetGuardrail` for step-budget enforcement
- `RateLimitGuardrail` for rolling dispatch throttling
- `CancellationGuardrail` for cooperative cancellation enforcement
- Task metadata introspection via `guardrail_name` and `guardrail_stage`

## Execution Model

Guardrails run in three phases:

1. `pre_check`
   Runs before any task work starts.
   Use this for policy validation, rate limiting, or early cancellation.

2. `mid_check`
   Runs before each retry attempt.
   Use this for dynamic cancellation or attempt-based policy.

3. `post_check`
   Runs after a task result is produced.
   Use this for output or step-consumption enforcement.

The first guardrail that rejects short-circuits execution and produces a
classified `TaskResult` failure with metadata.

## Quick Start

```rust,no_run
use std::sync::Arc;

use antikythera_core::application::agent::multi_agent::{
    AgentProfile, AgentTask, BudgetGuardrail, ExecutionMode, GuardrailChain,
    MultiAgentOrchestrator, TimeoutGuardrail,
};

# async fn demo<P: antikythera_core::infrastructure::model::ModelProvider + 'static>(
#     client: std::sync::Arc<antikythera_core::application::client::McpClient<P>>,
# ) {
let guardrails = GuardrailChain::new()
    .with_guardrail(Arc::new(TimeoutGuardrail::new(5_000).require_timeout()))
    .with_guardrail(Arc::new(
        BudgetGuardrail::new()
            .with_max_task_steps(8)
            .require_explicit_budget(),
    ));

let orchestrator = MultiAgentOrchestrator::new(client, ExecutionMode::Sequential)
    .register_agent(AgentProfile {
        id: "reviewer".into(),
        name: "Reviewer".into(),
        role: "code-review".into(),
        system_prompt: Some("You are a code reviewer".into()),
        max_steps: Some(8),
    })
    .with_guardrails(guardrails);

let result = orchestrator
    .dispatch(
        AgentTask::new("Review this patch")
            .with_timeout_ms(2_000)
            .with_budget_steps(6),
    )
    .await;

assert!(result.success || result.metadata.guardrail_name.is_some());
# }
```

## Built-In Guardrails

### TimeoutGuardrail

Validates `task.timeout_ms` before execution starts.

- `TimeoutGuardrail::new(max_timeout_ms)` sets the upper bound.
- `.require_timeout()` rejects tasks that omit `timeout_ms`.

Use this when the host requires explicit timeout discipline on all tasks.

### BudgetGuardrail

Controls how many steps a task may request or consume.

- `.with_max_task_steps(limit)` caps requested or observed steps
- `.require_explicit_budget()` requires `budget_steps` on every task
- `.allow_exhausted_orchestrator()` disables the shared-budget rejection path

This complements `OrchestratorBudget` instead of replacing it.

### RateLimitGuardrail

Limits how many tasks may start within a rolling time window.

```rust
use std::sync::Arc;
use antikythera_core::application::agent::multi_agent::RateLimitGuardrail;

let limit = Arc::new(RateLimitGuardrail::new(10, 60_000));
```

The guardrail keeps its own shared in-memory timestamp queue and rejects with
`ErrorKind::Transient` when the window is full.

### CancellationGuardrail

Rejects tasks when the orchestrator cancellation token has been triggered.

This is most useful in guardrail chains because it produces explicit
guardrail metadata instead of relying only on the built-in cancellation path.

## Custom Guardrails

Implement `TaskGuardrail` and override one or more lifecycle hooks.

```rust
use antikythera_core::application::agent::multi_agent::{
    AgentProfile, AgentTask, ErrorKind, GuardrailContext, GuardrailRejection,
    GuardrailStage, TaskGuardrail,
};

struct RequirePriorityMetadata;

impl TaskGuardrail for RequirePriorityMetadata {
    fn name(&self) -> &'static str {
        "require_priority"
    }

    fn pre_check(
        &self,
        task: &AgentTask,
        _profile: &AgentProfile,
        _context: &GuardrailContext,
    ) -> Result<(), GuardrailRejection> {
        if task.metadata.contains_key("priority") {
            Ok(())
        } else {
            Err(GuardrailRejection::new(
                self.name(),
                GuardrailStage::PreCheck,
                ErrorKind::Permanent,
                "priority metadata is required",
            ))
        }
    }
}
```

## Result Introspection

When a guardrail rejects a task, the returned `TaskResult` includes:

- `error_kind`
- `metadata.guardrail_name`
- `metadata.guardrail_stage`
- `metadata.routing_decision`
- `metadata.concurrency_wait_ms`
- `metadata.budget_exhausted` when relevant

Example:

```json
{
  "success": false,
  "error_kind": "budget_exhausted",
  "metadata": {
    "guardrail_name": "budget",
    "guardrail_stage": "pre_check",
    "budget_exhausted": true
  }
}
```

## Design Notes

- Guardrails are evaluated in registration order.
- The first rejection wins.
- Guardrails are cloned cheaply because the chain stores `Arc<dyn TaskGuardrail>`.
- Built-in guardrails are synchronous by design to avoid extra runtime overhead.
- Existing orchestration features such as retry, deadline, cancellation, and
  `OrchestratorBudget` remain active even when no guardrails are configured.

## Testing

This feature is covered by:

- 20 unit tests in `guardrails.rs`
- 3 orchestrator integration tests in `tests/multi_agent/guardrails_tests.rs`
- full `antikythera-core` library test pass
- strict clippy pass with `--all-features -D warnings`

## Backward Compatibility

Guardrails are additive and opt-in.

- Existing orchestrator code works unchanged.
- No existing public API was removed.
- New metadata fields on `TaskExecutionMetadata` are optional and defaulted.

## Related Types

- `MultiAgentOrchestrator`
- `OrchestratorBudget`
- `CancellationToken`
- `TaskRetryPolicy`
- `TaskExecutionMetadata`