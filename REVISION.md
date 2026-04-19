# Antikythera MCP Framework Revision
---

## Release Roadmap: v0.9.8 (Stabilization) and v0.9.9 (Production Hardening)

### v0.9.8 — **Stabilization & Core Foundation Hardening**

Target: Achieve production readiness for the core agent/tool-calling pipeline.

#### 1 Feature set: v0.9.8

**Priority 1: Advanced Context Management v1 (Foundation)**
- ✅ **Module**: `antikythera-core::application::context_management`
- **Scope**:
  - `ContextPolicy` struct: configurable truncation strategy (`KeepNewest`, `KeepBalanced`, `Summarize`)
  - `SummarizationStrategy` enum with rolling-summarization callback hooks
  - `RuntimeContextManager` for session-level context updates (policy mutation, summary injection)
  - Session-aware message filtering: keep system messages, respect `max_history_messages`, apply truncation strategy
  - Unit tests: 15+ for policy application, strategy selection, message filtering
  - Integration tests: multi-turn conversation with policy mutations
  - Documentation: `documentation/CONTEXT_MANAGEMENT.md` (usage examples, policy design patterns)
- **Backward compatibility**: All new types have sensible defaults; existing code works unchanged
- **Deliverables**: Code + tests + docs + clippy ✅

**Priority 2: MCP Tool-Calling Contract Hardening**
- ✅ **Module**: `antikythera-core::infrastructure::mcp::contract`
- **Scope**:
  - `ToolCallEnvelope` struct with validation (name, input schema, required fields)
  - `ToolResultEnvelope` struct with outcome determinism (success/error/partial_failure variants)
  - `ContractValidator` with strict envelope validation rules
  - Canonical error mapping: MCP tool errors → framework `ToolExecutionError` with deterministic retry logic
  - Unit tests: 20+ for envelope validation, error mapping, partial failure handling ✅ **20/20 passing**
  - Documentation: `documentation/MCP_CONTRACTS.md` (canonical shapes, error semantics) ✅
- **Backward compatibility**: Opt-in via existing imports (default on)
- **Deliverables**: Code + tests + docs + clippy ✅ **ALL COMPLETE**

**Priority 3: Multi-Agent Guardrails (Basic)**
- ✅ **Module**: `antikythera-core::application::agent::multi_agent::guardrails`
- **Scope**:
  - `TaskGuardrail` trait for custom execution rules
  - Built-in: `TimeoutGuardrail`, `BudgetGuardrail`, `RateLimitGuardrail`, `CancellationGuardrail`
  - Guardrail composition via `GuardrailChain`
  - Task pre-check, mid-check, and post-check hooks
  - Unit tests: 18+ for each guardrail type, composition, and enforcement
  - Integration tests: multi-agent execution with guardrails
  - Documentation: `documentation/GUARDRAILS.md` (policy examples, custom rules)
- **Backward compatibility**: Guardrails are opt-in; existing pipelines unaffected
- **Deliverables**: Code + tests + docs + clippy ✅

**Priority 4: Full Native CI Gates**
- ✅ **Deliverables**:
  - `.github/workflows/ci.yml`: `cargo test --workspace`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo fmt --check`
  - `.github/workflows/wasm-build.yml`: `cargo component build --target wasm32-wasip1`
  - Artifact verification: `antikythera_sdk.wasm` checksum validation
  - Branch protection rule: require CI pass before merge

**Priority 5: Host Integration Hooks**
- ✅ **Module**: `antikythera-core::application::hooks`
- **Scope**:
  - `AuthHook`: caller identity and permission propagation
  - `CorrelationHook`: correlation ID and request metadata flow
  - `PolicyDecisionHook`: model/tool access policy inputs
  - `TelemetryHook`: structured events, audit trails
  - Hook registry and middleware pattern
  - Unit tests: 12+ for hook registration, invocation, error handling
  - Documentation: `documentation/HOOKS.md` (hook lifecycle, integration patterns)
- **Backward compatibility**: Hooks are optional; no hooks = no overhead
- **Deliverables**: Code + tests + docs + clippy ✅

**Priority 6: Streaming Output (Phase 1)**
- ✅ **Module**: `antikythera-core::application::streaming`
- **Scope**:
  - `StreamingRequest` struct (token stream, event stream, mixed modes)
  - `StreamingResponse` trait for provider abstraction
  - `AgentEventStream` for intermediate agent events (token/tool/state)
  - CLI adapter: `antikythera` binary with `--stream` flag for token output
  - Unit tests: 10+ for streaming primitives
  - Integration tests: streaming from CLI and SDK
  - Documentation: `documentation/STREAMING.md` (modes, performance notes)
- **Backward compatibility**: All existing non-streaming APIs work unchanged
- **Deliverables**: Code + tests + docs + clippy ✅

#### 2 Quality gates for v0.9.8

- ✅ All tests pass: `cargo test --workspace`
- ✅ Strict clippy: `cargo clippy --workspace --all-features -- -D warnings`
- ✅ Formatting: `cargo fmt --check`
- ✅ Wasm build: `cargo component build --target wasm32-wasip1`
- ✅ Documentation: All new modules have `documentation/*.md` files with examples
- ✅ Backward compatibility: No breaking changes to public API surface
- ✅ Code coverage: All new modules have unit tests; integration tests exercise key flows

#### 3 Release notes: v0.9.8

- **Headline**: "Production-ready core agent and tool-calling pipeline with advanced context and guardrails"
- **Key additions**:
  - Advanced context management with configurable truncation strategies
  - Canonical MCP tool-calling contracts with strict validation
  - Multi-agent task guardrails (timeout, budget, rate-limit, cancellation)
  - Host integration hooks for auth, correlation, policy, and telemetry
  - Streaming output for token and agent events (Phase 1)
  - Full native CI gates
- **Migration guide**: None required; all changes are additive or opt-in

---

### v0.9.9 — **Production Hardening & Enterprise Integration**

Target: Production-grade observability, resilience, and enterprise deployment.

#### 4 Feature set: v0.9.9

**Priority 1: Observability & Metrics**
- Advanced health tracking with SLA metrics (p50/p95/p99 latencies)
- Metrics export hooks (Prometheus, CloudWatch, Datadog, etc.)
- Structured audit trails for policy decisions and tool execution
- Correlation ID propagation across agent/tool boundaries
- Distributed tracing support (OpenTelemetry hooks)

**Priority 2: Transport Plugin Architecture**
- `TransportPlugin` trait for custom MCP transports
- Built-in: WebSocket transport, gRPC transport stubs
- Service capability negotiation and version pinning
- Transport health and failover policies

**Priority 3: Enterprise Resilience**
- Circuit breaker for provider failures
- Bulkhead pattern for concurrent agent execution
- Graceful degradation and fallback policies
- Provider replica and load-balancing primitives

**Priority 4: Advanced Streaming (Phase 2)**
- Streaming tool results and intermediate outputs
- Streaming summaries from context management
- Buffered and unbuffered streaming modes
- Client-side streaming for large inputs

**Priority 5: Pre-1.0 Contract Freeze**
- Final API documentation and stability guarantees
- Deprecated API removal (if any)
- Version compatibility promise
- Semver enforcement

---

## 12. Implementation standards for 0.9.8+ work

All features added in 0.9.8+ releases MUST adhere to these standards:

1. **Documentation**:
   - Every module has `///` module-level doc comment with examples
   - Every public type has at least one example in doc comments
   - Feature flags and stability status are clearly marked (Stable / Experimental)
   - A corresponding `.md` file in `documentation/` explains the feature with use cases

2. **Unit Tests**:
   - Minimum 10+ tests per module
   - Tests cover happy path, error cases, edge cases, and integration with other modules
   - Tests are organized in a `tests` submodule
   - Test names clearly describe what is being tested

3. **Integration Tests**:
   - At least one integration test in `tests/` that exercises the feature end-to-end
   - Integration tests verify interaction with other subsystems (agents, config, etc.)

4. **Clippy Compliance**:
   - All code must pass `cargo clippy --all-targets -- -D warnings`
   - No `#[allow(...)]` attributes without justification comments
   - New warnings must be fixed before merging

5. **Backward Compatibility**:
   - No breaking changes to public APIs without major version bump
   - New types and traits have sensible defaults
   - Existing code continues to work without modification

6. **Code Review Checkpoints**:
   - All PRs must reference the roadmap item they implement
   - Implementation PRs are reviewed for architecture, testing, and documentation quality
   - Two approvals required before merge to main
