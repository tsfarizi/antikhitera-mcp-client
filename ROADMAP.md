# Antikythera v0.9.7 Implementation Roadmap

## ✅ Completed in v0.9.7

### 1. **Documentation Lane** — COMPLETED
- README fully updated with 2 deployment lanes (Native CLI + Server-side WASM component)
- Host Integration Contract documented
- Quick start examples for each lane
- Use cases and deployment targets clarified

### 2. **CI/CD Workflow** — COMPLETED
- GitHub Actions workflow added (`.github/workflows/ci.yml`)
- Includes: test, clippy, fmt checks
- Separate lane validation: default native lane + explicit component lane
- WASM component build verification
- Code coverage integration

### 3. **Version Bump** — COMPLETED
- Workspace version updated: 0.9.6 → 0.9.7
- README version updated

### 4. **Policy Audit Events** — COMPLETED
- New module: `antikythera-core/src/application/resilience/policy_audit.rs`
- Types: `PolicyAuditEvent`, `PolicyEventType` (9 event types)
- Sinks: `PolicyAuditSink` trait, `NoOpAuditSink`, `InMemoryAuditSink`
- Full test coverage for serialization and event capture
- **Usage:** Framework can now record policy decisions (tool access, rate limits, timeouts, health checks, etc.)

### 5. **Observability Hooks** — COMPLETED
- New module: `antikythera-core/src/application/observability.rs`
- Types: `CallerContext` (correlation ID, user ID, tenant ID, source, metadata)
- Types: `TelemetryEvent` (structured observability data)
- Trait: `ObservabilityHook` for host integration
- In-memory sink for testing: `InMemoryObservabilityHook`
- **Usage:** Hosts can now propagate caller context through agent runs, and subscribe to telemetry events

### 6. **Advanced Context Management Documentation** — COMPLETED
- New guide: `documentation/CONTEXT_MANAGEMENT.md`
- Comprehensive walkthrough of:
  - Token estimation (4-char rule, no tokenizer)
  - Truncation strategies (KeepNewest, KeepBalanced)
  - Rolling summarization
  - Per-provider/model policy overrides
  - Integration examples (CLI, WASM)
  - Testing guidance
  - Tuning recommendations

---

## ⏳ Deferred to v0.9.8+ (Out of Scope for v0.9.7)

### 7. **Streaming Output** (High Priority)
- Token streaming from LLM providers
- Intermediate agent event streaming
- Host-safe streaming contract for CLI/WASM embeddings
- **Reason deferred:** Requires substantial API changes to support streaming through WIT boundary and prepare/commit flow
- **Impact:** UX for CLI users and host embeddings will feel behind modern expectations without this
- **Planned for:** v0.9.8

### 8. **Multi-Agent Hardening** (Medium-High Priority)
- Cancellation and deadlines per task
- Concurrency and budget guardrails (semaphore-based worker limits)
- Partial-failure isolation and retry policies per task
- Richer introspection into routing and scheduling decisions
- **Reason deferred:** Requires substantial changes to `TaskScheduler` and `MultiAgentOrchestrator`
- **Impact:** Current orchestrator works but lacks production safety features for long-running multi-agent workflows
- **Planned for:** v0.9.8

---

## Test Results

**All tests pass with v0.9.7:**
```
test result: ok. 174 passed; 0 failed; 18 ignored; 0 measured
```

**CI/CD checks:**
- ✅ `cargo test --workspace` — PASS
- ✅ `cargo clippy --workspace --all-features` — PASS
- ✅ `cargo fmt --check` — PASS
- ✅ WASM component build (`wasm32-wasip1`) — PASS

---

## Documentation Updates

| Document | Change |
|----------|--------|
| `README.md` | Complete rewrite with lane documentation and host contract |
| `SUMMARY.md` | Added reference to `CONTEXT_MANAGEMENT.md` |
| `documentation/CONTEXT_MANAGEMENT.md` | NEW — Comprehensive context management guide |
| `.github/workflows/ci.yml` | NEW — GitHub Actions CI/CD workflow |

---

## Next Steps (v0.9.8)

1. **Implement Token Streaming**
   - Add streaming variants to `prepare_user_turn()` and `commit_llm_stream()`
   - Define WIT interface for streaming responses
   - Add CLI streaming support

2. **Multi-Agent Task Hardening**
   - Add `TaskCancellation` and deadlines to `AgentTask`
   - Implement per-task retry policies in `TaskScheduler`
   - Add partial-failure isolation for concurrent execution modes

3. **Host-Side Provider Adapters** (Optional)
   - Document how to implement provider-specific tool calling outside this repo
   - Example adapters for OpenAI, Anthropic, Gemini

---

## Architecture Decision Records (ADRs)

### ADR-v0.9.7-001: Policy Audit Events
- **Decision:** Add structured `PolicyAuditEvent` type for audit trail
- **Rationale:** Hosts need to log/audit policy decisions for compliance and debugging
- **Consequence:** Framework now exposes `PolicyAuditSink` trait for host integration
- **Status:** ✅ Implemented

### ADR-v0.9.7-002: Caller Context Propagation
- **Decision:** Add `CallerContext` with correlation ID, user ID, tenant ID, metadata
- **Rationale:** Multi-tenant SaaS hosts need to isolate and trace requests
- **Consequence:** Framework now exposes `TelemetryEvent` and `ObservabilityHook` trait
- **Status:** ✅ Implemented

### ADR-v0.9.7-003: Documentation-First Lane Clarity
- **Decision:** Lead with lane documentation in README; host contract documented upfront
- **Rationale:** Reduce confusion about deployment targets (native vs WASM)
- **Consequence:** README is now ~200 lines longer but crystal clear on lanes and responsibility boundaries
- **Status:** ✅ Completed

---

## Metrics

**Code additions:**
- New modules: 2 (`policy_audit.rs`, `observability.rs`)
- New documentation: 1 (`CONTEXT_MANAGEMENT.md`)
- Updated files: 5 (`README.md`, `SUMMARY.md`, `Cargo.toml`, `mod.rs` exports x2)
- GitHub Actions workflows: 1

**Test coverage:**
- New test cases: 6 (3 for policy audit, 3 for observability)
- All existing tests continue to pass: 174

**Breaking changes:** None
**Deprecated APIs:** None
**New public types:** 5 (`PolicyAuditEvent`, `PolicyEventType`, `CallerContext`, `TelemetryEvent`, `ObservabilityHook`)
