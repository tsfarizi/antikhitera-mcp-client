# Antikythera MCP Framework Revision

## 1. What this repository is

This repository is a Rust-based **MCP client framework**, not an MCP server.
It is designed to:

- connect to LLM providers
- connect to MCP tool servers over STDIO and HTTP
- run agent and tool-calling flows
- expose agent logic as a portable server-side WASM component (wasm32-wasip1)
- provide a native CLI for interactive and automated use

### Main crates

| Crate | Role | Current condition |
|---|---|---|
| `antikythera-core` | Core runtime: transports, providers, agents, config | Most mature and most important crate |
| `antikythera-sdk` | Public SDK surface: server-side WASM component bindings, config/session/agent helpers | Stable for native and component lanes |
| `antikythera-cli` | User-facing binaries: stdio chat, setup wizard, multi-agent harness | Fully connected to the core runtime |
| `antikythera-session` | Session history and import/export | Solid foundation |
| `antikythera-log` | Structured logging | Solid foundation |
| `scripts` and `wit` | WIT definition for server-side WASM component model (host imports/exports) | Present; component integration is active development |

---

## 2. Repository strengths

The repository already has several strong foundations:

- crate boundaries exist and are mostly understandable
- `antikythera-core` already contains the correct architectural center of gravity
- there is clear investment in server-side WASM (WASI component model) as the primary portability target
- documentation and build/release workflows are much better organized than before
- logging, config, and session capabilities are reusable and meaningful

This is **not** an early empty project. It already has real framework shape.

---

## 3. Major code consistency issues

### 3.1 `antikythera-cli` is inconsistent with `antikythera-core` ✅ RESOLVED

This is one of the largest consistency problems in the workspace.

**Changes made:**
- CLI LLM providers (`gemini.rs`, `ollama.rs`) now delegate to `antikythera_core::infrastructure::model::factory::ProviderFactory` instead of duplicating HTTP logic
- CLI config module (`cli/src/config/mod.rs`) replaced with re-exports from `antikythera_core::config::postcard_config`; both CLI and core now share `app.pc` as the single config file
- CLI main binary (`menu.rs`) updated from placeholder to a real thin adapter: loads config via `AppConfig::load()`, creates `DynamicModelProvider` and `McpClient`, then dispatches to `application::stdio::run` (STDIO mode) or the multi-agent orchestrator (multi-agent mode)

### 3.2 Some public-looking surfaces are still effectively stubs ✅ RESOLVED (FSM and registry)

Important examples:

- ~~the FSM-based agent path is not fully complete~~ **FIXED**: `transition()` fully implemented with all valid state transitions; `is_terminal()` fixed to include `FinalMessage`; 22 unit tests added
- ~~multi-agent is closer to a stub than a production orchestration runtime~~ **FULLY IMPLEMENTED (v0.9.6)**: Complete orchestration runtime with `ExecutionMode` (Auto/Sequential/Concurrent/Parallel), `TaskScheduler`, four `AgentRouter` implementations (`DirectRouter`, `RoundRobinRouter`, `FirstAvailableRouter`, `RoleRouter`), `MultiAgentOrchestrator`, pipeline execution, and CLI integration via `--mode multi-agent`
- JSON-RPC session flows are not fully wired into real session handling *(deferred — out of 3.x scope)*
- the server-side WASM component path (wasm32-wasip1) exposes structure, but the WIT-to-implementation wiring is not production-complete *(deferred — out of 3.x scope)*

**Impact**

The architecture appears more complete than the real runtime behavior.

### 3.3 Configuration is fragmented ✅ RESOLVED

There is more than one configuration path and shape across the workspace.


**Changes made:**
- `postcard_config::AppConfig` is now the single serialization format for all surfaces
- Added `PostcardAppConfig` type alias with disambiguation documentation
- `config/postcard_config.rs` module docs now explain the distinction between the serialised form and the runtime `app::AppConfig`
- `antikythera-cli` config module removed its divergent `CliConfig` struct and now re-exports from core; config file unified to `app.pc`

### 3.4 Feature flags and runtime reality are not fully aligned ✅ RESOLVED

Some features appear in manifests and docs before their runtime behavior is truly complete.

**Changes made:**
- `antikythera-core/Cargo.toml` now documents maturity for each feature flag:
  - `wizard` — marked ✅ STABLE
  - `multi-agent` — marked ✅ STABLE (v0.9.6): full orchestration; storage backends removed (storage is host's responsibility)
  - `wasm-runtime` — marked ✅ STABLE (v0.9.6): `WasmAgentRunner` via wasmtime; wasm-bindgen removed (host-side only)


---

## 3.5 Multi-agent orchestration and WASM runtime ✅ RESOLVED (v0.9.6)

**`multi-agent` feature — full orchestration runtime**

The `multi-agent` feature previously contained only `AgentRegistry` (CRUD for agent profiles). In v0.9.6 the following modules were added under `antikythera-core::application::agent::multi_agent`:

| Module | Contents |
|---|---|
| `execution` | `ExecutionMode` enum: `Auto` (tokio::spawn per task), `Sequential` (loop+await), `Concurrent` (FuturesUnordered, no spawn), `Parallel { workers }` (spawn + Semaphore) |
| `task` | `AgentTask` (builder pattern), `TaskResult` (success/failure), `PipelineResult` |
| `router` | `AgentRouter` trait, `DirectRouter`, `RoundRobinRouter`, `FirstAvailableRouter`, `RoleRouter` |
| `scheduler` | `TaskScheduler<T, F>` — generic over task type and executor closure; respects `ExecutionMode` |
| `orchestrator` | `MultiAgentOrchestrator<P>` — `dispatch()`, `dispatch_many()`, `pipeline()` |

`AgentProfile` extended with `system_prompt: Option<String>` and `max_steps: Option<usize>` (both `#[serde(default)]` for backward compatibility).

Feature flag cleaned up: `multi-agent = []` (no external dependencies required for core orchestration).
All persistent state storage (Redis, GCS, filesystem, databases) is the exclusive responsibility
of the HOST that embeds this framework. The WASM component only produces and consumes serialized
state blobs via WIT host imports (`save-state` / `load-state`); where and how that state is stored
is entirely up to the host language (Python, Go, TypeScript, Rust, etc.).

**`wasm-runtime` feature — wasmtime integration**

Added `antikythera-core::infrastructure::wasm::WasmAgentRunner`. The runner:
- Accepts raw WASM bytes or a file path
- Runs the module via wasmtime with a sandboxed `Store`
- Registers a `antikythera::call_llm_sync(ptr, len) -> i64` host import so WASM agents can call the LLM without managing threads
- Executes the module's `antikythera_run(ptr, len) -> i64` export
- Runs synchronous wasmtime code inside `tokio::task::spawn_blocking` so async callers are not blocked

WASM module ABI:
```
exports: antikythera_alloc(i32) -> i32
         antikythera_dealloc(i32, i32)
         antikythera_run(i32, i32) -> i64
imports: antikythera::call_llm_sync(i32, i32) -> i64
```

`wasm-runtime` feature now requires only `dep:wasmtime` (and `dep:anyhow`). `wasm-bindgen` and `wasm-bindgen-futures` were removed — this feature is for *hosting* WASM natively, not compiling to WASM.

**WIT interface updated**

`wit/antikythera.wit` gained a `multi-agent-runner` interface with six functions (`init-orchestrator`, `register-agent`, `dispatch-task`, `dispatch-tasks`, `pipeline-tasks`, `get-status`) that all exchange JSON strings for WASM compatibility. The `antikythera-agent` world exports this interface.

**CLI integration**

`--mode multi-agent` added to the `antikythera` binary with supporting flags:
- `--agents <path>` — JSON file containing agent profile array
- `--task <text>` — inline task text (or pipe from stdin)
- `--target-agent <id>` — route directly to a specific agent (uses `DirectRouter`)
- `--execution-mode <spec>` — `auto` (default), `sequential`, `concurrent`, `parallel:N`

Example:
```bash
mcp --mode multi-agent --agents agents.json --task "Review this code" --execution-mode parallel:4
```


---

## 4. Architectural mismatches

### 4.1 The modular architecture is directionally correct, but boundaries are not strict enough ✅ RESOLVED

The crate split is good in principle, but discipline is not fully enforced:

- CLI should be a thin adapter over the core runtime
- SDK should clearly separate server-side WASM component, native C FFI, and native Rust lanes
- The `wasm` SDK feature (browser WASM via `wasm-bindgen`) is a different target from the primary
  server-side WASM component (`component` feature, `wasm32-wasip1`); these two are conflated
  in the current feature defaults
- config should have a single source of truth

**Changes made:**
- `antikythera-sdk` default feature set changed to `default = ["single-agent"]` so browser WASM is no longer implicitly enabled
- Browser WASM (`wasm`) and server-side WASM component (`component`) are now explicit opt-in lanes
- Server-side WASM modules in SDK (`component` and `wasm_agent`) are now exported only when `feature = "component"` is enabled
- Config remains on the single canonical path (`postcard_config::AppConfig`) already established in section 3.3

### 4.2 The public SDK surface is broader than the truly stable implementation surface ✅ RESOLVED (lane-gated exports)

`antikythera-sdk` exposes many modules:

- config
- session
- agents
- servers
- JSON schema
- server-side WASM component interface (WIT)
- browser WASM client (secondary target, wasm-bindgen)
- native C FFI

But these surfaces are not all equally mature.

**Changes made:**
- Lane-specific SDK exports are now feature-gated so consumers only see the surface they explicitly opt into
- `wasm_agent` and `component` surfaces are hidden from default/native SDK builds unless `component` is enabled
- Default API now tracks the native lane more closely, reducing accidental reliance on secondary targets

### 4.3 Native and server-side WASM component are separate product lanes ✅ RESOLVED

The two actual deployment lanes are:

- **native runtime** — compiled natively, full tokio async, HTTP providers, CLI binary
- **server-side WASM component** — compiled to `wasm32-wasip1`, hosted by an embedding process
  (Rust/Python/Go/etc.) via wasmtime; host handles LLM calls through WIT imports; this is the
  primary WASM target for flexibility and portability without per-language runtime setup

Browser WASM (`wasm-bindgen`) and C FFI (`cdylib`, `extern "C"` exports) have been removed.
Hosts embedding the framework are responsible for any additional interface layers they require.

**Changes made:**
- Established the final contract that model API invocation belongs to the embedding host, not to the framework runtime itself
- `ModelRequest` / `ModelResponse` are now serializable host-boundary contracts
- Added host-delegating model transport primitives (`HostModelTransport`, `HostModelClient`, `HostModelResponse`)
- `ModelError::Network` variant changed to use a plain `String` message instead of `reqwest::Error`,
  making the type fully WASM-safe
- `DynamicModelProvider` gained a push-based `register()` / `new()` API that is always compiled
  (usable in WASM with stub/mock clients)
- Added a two-phase chat flow in core/SDK: framework prepares messages and session context first, host performs the LLM call, then host commits the response back into framework-managed history
- Host responses may now be plain text or a structured assistant message; both are accepted by the framework
- `http-providers` remains only as a deprecated compatibility flag and no longer defines the primary runtime path
- `antikythera-sdk` default features now target native lane only (`default = ["single-agent"]`); browser WASM and server-side component are explicit opt-in lanes
- `antikythera-sdk` lane-specific exports are now strictly gated: `wasm_agent` and `component` are exported only with `feature = "component"`
- Server-side WASM component builds (`cargo component build --target wasm32-wasip1`) are now clean: no HTTP deps

---

## 5. Pre-1.0 stabilization status after scope reduction

### ✅ 5.1 Radical scope simplification — COMPLETED (Current Session)

**COMPLETED:** Browser WASM, C FFI, and REST API have been completely removed.

**Changes made:**
- Removed `wasm` feature (browser WASM) from `antikythera-sdk`
- Removed `ffi` feature (C FFI exports) from `antikythera-sdk`
- Deleted `infrastructure/server/` (REST HTTP server)
- Deleted `infrastructure/rpc/` (JSON-RPC endpoints)
- Removed `RunMode::Rest` and `RunMode::All` from CLI
- Removed all REST API dependencies (axum, tower-http, utoipa-swagger-ui)
- CLI (`antikythera` binary) now supports only:
  - `--mode stdio` (interactive TUI)
  - `--mode setup` (configuration wizard)
  - `--mode multi-agent` (orchestrator harness)

**Result:**
- Two focused deployment lanes: **native CLI** and **server-side WASM component (wasm32-wasip1)**
- Host owns the interface layer (REST, gRPC, WebSocket, etc.)
- Codebase dramatically simplified (~700 lines of REST/FFI infrastructure deleted)
- All 82 tests continue to pass; no regressions

### ⏳ 5.2 Native CI quality gates — STILL PENDING

With REST API removed, the build matrix is clean and ready for CI gates.

**Next step:**
- Add GitHub Actions workflow for:
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-features`
  - `cargo fmt --check`

### ✅ 5.3 Final config contract — CONFIRMED

The `postcard_config::AppConfig` format is now the sole canonical configuration across all surfaces.

**Status:**
- ✅ Config is unified (`app.pc`)
- ✅ No REST server config bloat
- ✅ Ready to document as stable

### ✅ 5.4 Final API contract — GREATLY SIMPLIFIED

After radical simplification, the API surface is crystal clear:

| Deployment Lane | Status | API Surface |
|---|---|---|
| **Server-side WASM component** | ✅ PRIMARY | WIT imports/exports; host performs model calls, WASM prepares/consumes message state |
| **Native CLI** | ✅ ACTIVE | TUI (stdio), Setup wizard, MultiAgent orchestrator |
| **Native SDK** | ✅ STABLE | `McpClient`, `PreparedChatTurn`, host transport injection, `MultiAgentOrchestrator`, logging, config |
| **Browser WASM** | ❌ REMOVED | N/A |
| **C FFI** | ❌ REMOVED | N/A |
| **REST API** | ❌ REMOVED | N/A |

**Next step:**
- Document final contract in README

**Implemented now (host-driven contract runtime):**
- ✅ Added explicit two-phase host flow in SDK/agent runner:
  - `prepare_user_turn(request_json)` — builds session-aware message payload for host model invocation
  - `commit_llm_response(prepared_turn_json, llm_response_json)` — commits host response into WASM-managed history/state
- ✅ Added session runtime helpers for host integration:
  - `init_agent_runner`, `process_llm_response_for_session`, `process_tool_result_for_session`, `get_agent_state`, `reset_agent_session`
- ✅ Host responses can be plain text or structured JSON action payloads; both are accepted and normalized into framework state
- ✅ Session continuity remains internal to framework/WASM (`session_id`, message history, step tracking), while model HTTP/API invocation remains fully host-owned

### ✅ 5.5 Runtime resilience — COMPLETED

With scope simplified, resilience patterns have been added cleanly.

**Implemented:**
- ✅ **Retry + exponential back-off** — `RetryPolicy` with configurable `max_attempts`, `initial_delay_ms`, `max_delay_ms`, `backoff_factor`; `with_retry` / `with_retry_if` async executors in `application::resilience::retry`.
- ✅ **Timeout policies** — `TimeoutPolicy` with per-call `llm_timeout_ms` and `tool_timeout_ms`; `llm_duration()` / `tool_duration()` helpers for direct use with `tokio::time::timeout`.
- ✅ **Context-window management** — `TokenEstimator` (1 token ≈ 4 chars, no tokenizer dep); `ContextWindowPolicy`; `prune_messages` (retains system messages + newest history, always keeps `min_history_messages`).
- ✅ **Health / status tracking** — `HealthStatus` enum (`healthy` / `degraded` / `unhealthy`); `ComponentHealth` (error rate, EMA latency, last error); `HealthTracker` with `record_success` / `record_failure`, `overall_status`, `snapshot_json`.
- ✅ **WIT / FFI exposure** — `ResilienceManager` facade with JSON-in/JSON-out methods mirrors the `resilience` WIT interface exported by the WASM component (6 functions: `get-config`, `set-config`, `get-health`, `reset-health`, `estimate-tokens`, `prune-messages`).
- ✅ **Unit tests** — 38 inline tests across `policy`, `retry`, `context_window`, `health`, and `mod`.
- ✅ **Integration tests** — 11 tests in `tests/resilience/resilience_tests.rs` validating the public API from an external crate.
- ✅ **Documentation** — `documentation/RESILIENCE.md` covering all submodules, WIT interface, and usage examples.
- ✅ **Crate-level re-exports** — all types available at `antikythera_core::*` root.

---

## 6. Remaining features that fit the current product scope

The following items assume the current product vision:

- no embedded REST module inside the framework
- no browser WASM and no C FFI lane
- any REST, gRPC, WebSocket, or custom API surface belongs to the embedding host, not to Antikythera itself

### 6.1 Streaming responses

This is essential.

The project still needs:

- token streaming from providers
- intermediate agent event streaming
- a host-safe streaming contract for CLI and WASM component embeddings

Without this, UX for CLI users and embedding hosts will feel behind modern expectations.

### 6.2 Advanced context management

Basic token estimation, pruning, retry, timeout, and health tracking already landed in section 5.5.
The remaining gap is advanced context management.

The project needs:

- rolling summarization
- runtime/session policy updates via MCP client control plane
- configurable truncation and summarization strategy per agent or session
- host-visible summary handoff for component embeddings

Without this, long conversations will still degrade even though hard token-limit failures are reduced.

### 6.3 MCP-native tool calling contract

The current agent flow still leans heavily on internal JSON conventions.

For a strong 1.0 MCP client framework, tool calling should be centered on one
canonical MCP contract:

- strict tool call envelope validation
- strict tool result envelope validation
- deterministic retry and error-mapping semantics for MCP tool failures
- generic host normalization only at the boundary before committing to MCP contract

This keeps tool execution behavior consistent regardless of upstream model/provider API shape.

### 6.4 Host-facing policy and integration hooks

Because the host owns the interface layer, the framework still needs clearer primitives for:

- auth and caller-context propagation from host into an agent run
- request metadata and correlation IDs
- model and tool access policy inputs
- auditable decision events and policy failures

Without that, each host embedding will rebuild policy wiring ad hoc.

### 6.5 Telemetry and observability hooks

At minimum:

- health snapshots exportable to the host
- metrics-friendly counters and timers
- correlation ID and session ID propagation
- structured telemetry events

These should be exposed as framework hooks and data surfaces, not as built-in HTTP endpoints.

### 6.6 Transport plugin architecture

The current transport layer is still fairly fixed.
A strong framework would allow extension for:

- WebSocket transport
- custom internal bridges
- enterprise-specific transports
- clearer transport capability negotiation

### 6.7 Multi-agent production hardening

The orchestration runtime now exists and is public.
The remaining gap is production hardening:

- cancellation and deadlines per task
- concurrency and budget guardrails
- partial-failure isolation and retry policies
- richer introspection into routing and scheduling decisions

---

## 7. What feels least correct architecturally right now

If the biggest mismatches are ranked:

1. ~~**CLI duplicate universe**~~ ✅ Resolved
2. ~~**config fragmentation**~~ ✅ Resolved
3. ~~**public surface broader than implementation maturity**~~ ✅ Resolved (scope reduced to two lanes)
4. ~~**feature flags imply readiness that runtime does not always match**~~ ✅ Resolved
5. ~~**native/browser/component lanes are not yet strongly formalized**~~ ✅ Resolved (browser WASM and C FFI removed)

---

## 8. Recommended implementation roadmap

## Phase 1 — Clean up the foundation ✅ COMPLETED

1. ✅ Make `antikythera-core` the single runtime source of truth.
2. ✅ Remove duplication from `antikythera-cli`; keep CLI as a thin adapter.
3. ✅ Unify the config system.
4. ✅ Decide which public features are truly supported.
5. Add full native CI gates.

## Phase 2 — Define product lanes clearly ✅ COMPLETED

Two deployment lanes:

- **native** — full tokio async, HTTP providers, CLI binary
- **server-side WASM component** — `wasm32-wasip1` WASI, WIT imports/exports, hosted via wasmtime;
  host language calls the `.wasm` binary via the component ABI

## Phase 3 — Add the remaining features that fit the current scope

Implement:

- streaming output for CLI and WASM component embeddings
- advanced context management and summarization
- MCP tool calling contract hardening (canonical envelopes and error mapping)
- host-facing observability and policy hooks
- transport extensibility and multi-agent hardening

## Phase 4 — Finalize the 1.0 contract

Before 1.0, decide clearly:

- which APIs are officially supported
- what the final config contract is
- how release/versioning compatibility works
- which features are postponed or removed

---

## 9. Highest priorities

### Critical priorities

1. ✅ Make the CLI genuinely usable
2. Add full native CI
3. ✅ Finalize one config system
4. ✅ Remove duplicate architecture in the CLI
5. ✅ Finalize deployment lane scope (native + server-side WASM component)

### High priorities

6. Add streaming for CLI and host embeddings
7. Extend context management with summarization and runtime/session-level policy controls
8. Harden MCP tool-calling contracts and error mapping
9. Add host-facing observability hooks and harden multi-agent execution

---

## 10. Final conclusion

This repository has a strong foundation and a clear product direction, targeting a **focused pre-1.0 framework** with two well-defined deployment lanes.

All foundational consistency issues are resolved:

- architecture has a single runtime source of truth (`antikythera-core`)
- boundaries between crates are now strict and consistent
- the CLI is a thin adapter over core
- config is unified
- scope is narrowed to native and server-side WASM component lanes

The path forward is to finish the remaining stabilization work and add only the features that match the current scope: streaming, advanced context management, MCP-native tool-calling contracts, host-facing observability, and multi-agent hardening.

Antikythera should not grow an embedded REST module again. If a product needs REST, gRPC, WebSocket, or another interface layer, that surface should live in the host that embeds the framework or consumes the WASM component.

**Consolidate, narrow, then add only what is truly required.**
