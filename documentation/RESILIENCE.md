# Runtime Resilience

This document describes the runtime resilience system added in `antikythera-core::application::resilience`.  The module provides retry back-off, timeout policies, context-window management, and health tracking — and exposes all of this through the WIT `resilience` interface so any WASM component host can configure and observe the agent's runtime behaviour.

---

## Overview

| Submodule         | What it provides                                                        |
|-------------------|-------------------------------------------------------------------------|
| `policy`          | `RetryPolicy`, `TimeoutPolicy`, `ResilienceConfig`                      |
| `retry`           | `with_retry` / `with_retry_if` — async retry executor with back-off     |
| `context_window`  | `TokenEstimator`, `ContextWindowPolicy`, `prune_messages`               |
| `health`          | `HealthStatus`, `ComponentHealth`, `HealthTracker`                      |
| `mod` (root)      | `ResilienceManager` — unified facade mirroring the WIT interface        |

---

## Retry Policy and Exponential Back-off

### `RetryPolicy`

```rust
pub struct RetryPolicy {
    pub max_attempts:     u32,   // default: 3  (1 = no retries)
    pub initial_delay_ms: u64,   // default: 200 ms
    pub max_delay_ms:     u64,   // default: 10 000 ms
    pub backoff_factor:   f64,   // default: 2.0
}
```

The delay before attempt `n` (0-based) is:

$$d_n = \min\left(\text{initial\_delay\_ms} \times \text{backoff\_factor}^n,\ \text{max\_delay\_ms}\right)$$

| Attempt | Delay (defaults) |
|---------|-----------------|
| 0       | 200 ms          |
| 1       | 400 ms          |
| 2       | 800 ms          |
| 3       | 1 600 ms        |
| 4       | 3 200 ms        |

#### Preset constructors

```rust
RetryPolicy::no_retry()    // max_attempts = 1
RetryPolicy::default()     // max_attempts = 3
RetryPolicy::aggressive()  // max_attempts = 5
```

### Retry executor

```rust
// Retry on every error
with_retry(&policy, || async { call_llm().await }).await

// Retry only on transient errors
with_retry_if(&policy, || async { call_llm().await }, |e| e.is_network()).await
```

The `should_retry` predicate receives a reference to the error value after each failure.  Return `true` to allow a retry, `false` to propagate immediately.

**Recommended usage for LLM calls:**

```rust
use antikythera_core::resilience::{RetryPolicy, with_retry_if};
use antikythera_core::infrastructure::model::ModelError;

let policy = RetryPolicy::default();
let result = with_retry_if(
    &policy,
    || async { provider.chat(request.clone()).await },
    |e| matches!(e, ModelError::Network { .. }),
).await;
```

---

## Timeout Policy

```rust
pub struct TimeoutPolicy {
    pub llm_timeout_ms:  u64,  // default: 30 000 ms
    pub tool_timeout_ms: u64,  // default: 10 000 ms
}
```

Wrap calls with `tokio::time::timeout` using the pre-built duration helpers:

```rust
use tokio::time::timeout;
use antikythera_core::resilience::TimeoutPolicy;

let policy = TimeoutPolicy::default();

// LLM call
let response = timeout(policy.llm_duration(), provider.chat(req)).await
    .map_err(|_| "LLM call timed out")?;

// Tool call
let result = timeout(policy.tool_duration(), execute_tool(args)).await
    .map_err(|_| "Tool call timed out")?;
```

---

## Context-Window Management

### Token estimation

`TokenEstimator` uses the rule of thumb **1 token ≈ 4 characters** with a minimum of 1 token.  No tokenizer library is required.  Accuracy is ±30% for typical English prompts — sufficient for proactive pruning without adding ML dependencies.

```rust
use antikythera_core::resilience::TokenEstimator;

let tokens = TokenEstimator::estimate_text("Hello, world!");          // text
let msg_tokens = TokenEstimator::estimate_message(&chat_message);     // single message
let total = TokenEstimator::estimate_messages(&history);              // full history
```

### `ContextWindowPolicy`

```rust
pub struct ContextWindowPolicy {
    pub max_tokens:           usize,  // default: 8 192
    pub reserve_for_response: usize,  // default: 1 024
    pub min_history_messages: usize,  // default: 2
}
```

`message_budget()` returns the effective token budget for the message list:

$$\text{budget} = \text{max\_tokens} - \text{reserve\_for\_response}$$

### `prune_messages`

```rust
use antikythera_core::resilience::{ContextWindowPolicy, prune_messages};

let policy = ContextWindowPolicy::default();
let pruned = prune_messages(&history, &policy);
```

**Pruning strategy:**

1. System messages are always retained (never pruned).
2. Non-system messages are accumulated newest → oldest.
3. The oldest messages are dropped once the budget is exceeded.
4. At least `min_history_messages` non-system messages are always kept, even if they push the total above budget.

---

## Health Tracking

### Status thresholds

| Error rate       | `HealthStatus`  |
|-----------------|-----------------|
| 0 %             | `Healthy`       |
| > 0 % and < 50 %| `Degraded`      |
| ≥ 50 %          | `Unhealthy`     |

### `HealthTracker`

```rust
use antikythera_core::resilience::{HealthTracker, HealthStatus};

let mut tracker = HealthTracker::new();

// Record outcomes
tracker.record_success("gemini-flash", 320);  // latency_ms
tracker.record_failure("gemini-flash", "HTTP 503");

// Query
let health = tracker.health_of("gemini-flash").unwrap();
println!("status: {}", health.status);           // "degraded"
println!("error_rate: {:.1}%", health.error_rate * 100.0);

// Overall
let overall = tracker.overall_status(); // worst component status

// JSON snapshot for the host
let json = tracker.snapshot_json();
```

### `ComponentHealth` JSON schema

```json
{
  "component_id":    "gemini-flash",
  "status":          "healthy",
  "total_calls":     42,
  "successful_calls": 41,
  "error_rate":      0.024,
  "avg_latency_ms":  312.5,
  "last_error":      null
}
```

---

## ResilienceManager

`ResilienceManager` bundles a `ResilienceConfig` and a `HealthTracker` into a single object and provides JSON-in / JSON-out methods that map 1-to-1 onto the WIT `resilience` interface:

| Rust method                      | WIT export             |
|----------------------------------|------------------------|
| `get_config_json()`              | `get-config`           |
| `set_config_from_json(json)`     | `set-config`           |
| `get_health_json()`              | `get-health`           |
| `reset_health()`                 | `reset-health`         |
| `estimate_tokens(text)`          | `estimate-tokens`      |
| `prune_messages_json(…)`         | `prune-messages`       |

```rust
use antikythera_core::resilience::{ResilienceManager, ResilienceConfig, RetryPolicy};

let mut mgr = ResilienceManager::new();

// Configure via JSON (WIT set-config)
mgr.set_config_from_json(r#"{
    "retry":   {"max_attempts": 5, "initial_delay_ms": 100, "max_delay_ms": 5000, "backoff_factor": 2.0},
    "timeout": {"llm_timeout_ms": 20000, "tool_timeout_ms": 5000}
}"#).unwrap();

// Record health
mgr.health_mut().record_success("gemini-flash", 280);

// Query health (WIT get-health)
let health_json: String = mgr.get_health_json();
```

---

## WIT Interface

The `resilience` interface is exported by every WASM component built from this framework.  Host languages call it through the WASM component boundary to configure runtime behaviour and to observe health without recompiling the component.

```wit
interface resilience {
    get-config()  -> string;
    set-config(config-json: string) -> result<bool, string>;
    get-health()  -> string;
    reset-health();
    estimate-tokens(text: string) -> u32;
    prune-messages(messages-json: string, max-tokens: u32, reserve-tokens: u32)
        -> result<string, string>;
}
```

Full WIT definition: [`wit/antikythera.wit`](../wit/antikythera.wit)

### Example host call (Python pseudocode)

```python
# Load the component
store, instance = load_component("agent.wasm")
resilience = instance.exports.resilience

# Increase retry budget
resilience.set_config(json.dumps({
    "retry":   {"max_attempts": 5, "initial_delay_ms": 200, "max_delay_ms": 10000, "backoff_factor": 2.0},
    "timeout": {"llm_timeout_ms": 30000, "tool_timeout_ms": 10000},
}))

# Run the agent …

# After the run, inspect component health
health = json.loads(resilience.get_health())
for component in health:
    print(f"{component['component_id']}: {component['status']} "
          f"(error_rate={component['error_rate']:.1%})")
```

---

## Test Coverage

| Test file                                    | Tests |
|----------------------------------------------|-------|
| `resilience/policy.rs` (unit)                | 7     |
| `resilience/retry.rs` (unit, async)          | 5     |
| `resilience/context_window.rs` (unit)        | 10    |
| `resilience/health.rs` (unit)                | 8     |
| `resilience/mod.rs` (unit, ResilienceManager)| 8     |
| `tests/resilience/resilience_tests.rs` (integration) | 11    |
| **Total**                                    | **49** |

Run all resilience tests:

```bash
# Unit tests only (fast, no network)
cargo test --package antikythera-core -- resilience

# Integration tests
cargo test --package antikythera-tests --test resilience_tests
```

---

## Crate-level re-exports

All public types are re-exported at the `antikythera-core` crate root:

```rust
use antikythera_core::{
    // Policies
    ResilienceConfig, RetryPolicy, TimeoutPolicy, ContextWindowPolicy,
    // Execution
    with_retry, with_retry_if,
    // Context window
    TokenEstimator, prune_messages,
    // Health
    HealthStatus, ComponentHealth, HealthTracker,
    // Unified facade
    ResilienceManager,
    // Module path access
    resilience,
};
```
