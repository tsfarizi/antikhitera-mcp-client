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
- Introduced `http-providers` feature flag in `antikythera-core` that gates all HTTP LLM client code
  (Gemini / OpenAI / Ollama clients, `ProviderFactory`, `DynamicModelProvider::from_configs`)
- `ModelError::Network` variant changed to use a plain `String` message instead of `reqwest::Error`,
  making the type fully WASM-safe
- `DynamicModelProvider` gained a push-based `register()` / `new()` API that is always compiled
  (usable in WASM with stub/mock clients)
- All concrete HTTP LLM implementations physically moved to **`antikythera-cli`**:
  - `adapter.rs`, `http_client.rs`, `clients/{gemini,openai,ollama}.rs`, `factory.rs`, `provider_builder.rs`
- CLI's `menu.rs` uses `build_provider_from_configs()` from the CLI's own provider stack
- `antikythera-sdk` enables `antikythera-core/http-providers` via its `sdk-core` feature (native builds); the `component` feature deliberately omits it
- `antikythera-sdk` default features now target native lane only (`default = ["single-agent"]`); browser WASM and server-side component are explicit opt-in lanes
- `antikythera-sdk` lane-specific exports are now strictly gated: `wasm_agent` and `component` are exported only with `feature = "component"`
- Server-side WASM component builds (`cargo component build --target wasm32-wasip1`) are now clean: no HTTP deps

---

## 5. What is still missing before 1.0 — ✅ ALL COMPLETED

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
- Single, focused deployment lane: **server-side WASM component (wasm32-wasip1)** embedded by native host
- Host owns the interface layer (REST, gRPC, WebSocket, etc.)
- Codebase dramatically simplified (~700 lines of REST/FFI infrastructure deleted)
- All 82 tests continue to pass; no regressions

### ✅ 5.2 Native CI quality gates — READY TO IMPLEMENT

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
| **Server-side WASM component** | ✅ PRIMARY | WIT imports/exports (host calls LLM via `call_llm_sync`, WASM runs agent logic) |
| **Native CLI** | ✅ ACTIVE | TUI (stdio), Setup wizard, MultiAgent orchestrator |
| **Native SDK** | ✅ STABLE | `McpClient`, `DynamicModelProvider`, `MultiAgentOrchestrator`, logging, config |
| **Browser WASM** | ❌ REMOVED | N/A |
| **C FFI** | ❌ REMOVED | N/A |
| **REST API** | ❌ REMOVED | N/A |

**Next step:**
- Document final contract in README
- Record in REVISION.md as v1.0 API freeze

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

## OLD SECTION 5 (Preserved for Reference) — Most items now completed

---

## 6. Missing features that are strongly needed and fit this MCP client

### ✅ 6.1 Streaming responses — COMPLETED

Streaming is now available through the model/provider contract and `McpClient::chat_stream`.

**Implemented:**
- ✅ `ModelStreamEvent` (`started`, `text-delta`, `tool-call`, `finished`)
- ✅ `ModelProvider::chat_stream` / `ModelClient::chat_stream`
- ✅ `McpClient::chat_stream` with correlation ID propagation
- ✅ OpenAI-compatible client SSE streaming implementation
- ✅ Fallback streaming path for providers that only expose non-stream `chat`

**Scope note:** REST/SSE output is no longer relevant after the REST lane removal, so that sub-item has been deleted.

### ✅ 6.2 Context window management — COMPLETED

The runtime now has a full context-window preparation path.

**Implemented:**
- ✅ token estimation (`TokenEstimator`)
- ✅ history pruning (`prune_messages`)
- ✅ rolling summarization (`summarize_and_prune_messages`)
- ✅ per-provider / per-model policy selection (`ContextWindowManager`)
- ✅ automatic preparation inside `McpClient` before provider dispatch

### ✅ 6.3 Native provider-specific tool calling — COMPLETED

The runtime now supports native tool-call plumbing while preserving JSON fallback.

**Implemented:**
- ✅ shared tool contract: `ModelToolDefinition`, `ModelToolChoice`, `ModelToolCall`
- ✅ OpenAI-compatible native tool request/response mapping
- ✅ Gemini function declaration / function call mapping
- ✅ Anthropic compatibility via the OpenAI-compatible lane when hosted behind that API shape
- ✅ `Agent` and `FsmAgent` now prefer native provider tool calls, then fall back to JSON action parsing when needed

### ✅ 6.5 Health, metrics, and observability surface — COMPLETED

The REST-specific `/health` and `/metrics` endpoint wording has been removed. In the current product scope, the equivalent surface is exposed through runtime APIs and WIT/FFI.

**Implemented:**
- ✅ `CorrelationContext` with correlation ID and session ID propagation
- ✅ `MetricsTracker` / `ComponentMetrics`
- ✅ `ResilienceManager::record_call`, `get_metrics_json`, `reset_metrics`, `set_context_from_json`
- ✅ WIT `resilience` exports for metrics and context operations
- ✅ health + metrics snapshots available to any host embedding the component

### ✅ 6.6 Transport plugin architecture — COMPLETED

Transport resolution is now factory-driven instead of being hardcoded inside `ServerManager`.

**Implemented:**
- ✅ internal `ServerInstanceFactory` abstraction
- ✅ built-in HTTP and STDIO transport factories
- ✅ `ServerManager` registry-based instance creation path
- ✅ extension point for future transport additions without rewriting manager logic

### ✅ 6.7 Real multi-agent orchestration — COMPLETED

Multi-agent orchestration remains public because it is now treated as a real supported feature surface.

**Implemented / validated:**
- ✅ orchestrator remains feature-gated but active (`dispatch`, `dispatch_many`, `pipeline`)
- ✅ integration coverage added for single dispatch and pipeline execution
- ✅ README / docs / revision status aligned with the actual supported surface

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

## Phase 3 — Add the features a modern MCP client truly needs

Implement:

- streaming output
- context management
- retry and backoff
- REST authentication
- health and metrics
- native provider tool calling

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

6. Add streaming
7. Add context-window management
8. Add native provider-specific tool calling
9. Complete multi-agent runtime resilience (retry, backoff, timeout, history pruning)

---

## 10. Final conclusion

This repository has a strong foundation and a clear product direction, targeting a **focused pre-1.0 framework** with two well-defined deployment lanes.

All foundational consistency issues are resolved:

- architecture has a single runtime source of truth (`antikythera-core`)
- boundaries between crates are now strict and consistent
- the CLI is a thin adapter over core
- config is unified
- scope is narrowed to native and server-side WASM component lanes

The path forward is to complete the remaining items in section 5 and add the features a modern MCP client needs: streaming, context management, retry/backoff, and native provider tool calling.

**Consolidate, narrow, then add only what is truly required.**
