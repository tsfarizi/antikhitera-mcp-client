# Antikythera MCP Framework Revision

## 1. What this repository is

This repository is a Rust-based **MCP client framework**, not an MCP server.
It is designed to:

- connect to LLM providers
- connect to MCP tool servers over STDIO and HTTP
- run agent/tool-calling flows
- expose capabilities through SDK, WASM, and REST surfaces

### Main crates

| Crate | Role | Current condition |
|---|---|---|
| `antikythera-core` | Core runtime: transports, providers, agents, config, REST server | Most mature and most important crate |
| `antikythera-sdk` | Public SDK surface: server-side WASM component bindings, C FFI, config/session/server/agent helpers | Broad surface, but not equally mature everywhere |
| `antikythera-cli` | User-facing binaries | Not fully connected to the real runtime yet |
| `antikythera-session` | Session history and import/export | Solid foundation |
| `antikythera-log` | Structured logging | Solid foundation |
| `scripts` and `wit` | WIT definition for server-side WASM component model (host imports/exports) | Present, but component integration is not fully mature |

---

## 2. Repository strengths

The repository already has several strong foundations:

- crate boundaries exist and are mostly understandable
- `antikythera-core` already contains the correct architectural center of gravity
- there is clear investment in server-side WASM (WASI component model) and FFI hosting support
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
- CLI main binary (`menu.rs`) updated from placeholder to a real thin adapter: loads config via `AppConfig::load()`, creates `DynamicModelProvider` and `McpClient`, then dispatches to `application::stdio::run` (STDIO mode) or `infrastructure::server::serve` (REST mode)

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

### 4.1 The modular architecture is directionally correct, but boundaries are not strict enough

The crate split is good in principle, but discipline is not fully enforced:

- CLI should be a thin adapter over the core runtime
- SDK should clearly separate server-side WASM component, native C FFI, and native Rust lanes
- The `wasm` SDK feature (browser WASM via `wasm-bindgen`) is a different target from the primary
  server-side WASM component (`component` feature, `wasm32-wasip1`); these two are conflated
  in the current feature defaults
- config should have a single source of truth

Right now those boundaries still leak.

### 4.2 The public SDK surface is broader than the truly stable implementation surface

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

**Impact**

It is difficult to say what the actual stable product API is.

### 4.3 Native, server-side WASM component, and C FFI lanes are not yet fully treated as separate product lanes ✅ RESOLVED

The three actual deployment lanes are:

- **native runtime** — compiled natively, full tokio async, HTTP providers
- **server-side WASM component** — compiled to `wasm32-wasip1`, hosted by an embedding process
  (Rust/Python/Go/etc.) via wasmtime; host handles LLM calls through WIT imports; this is the
  primary WASM target for flexibility and portability without per-language runtime setup
- **native C FFI** — compiled as `cdylib`, called from C/C++/Python via `extern "C"` exports

The browser WASM (`wasm` feature, `wasm-bindgen`) is a **secondary optional** path, not a core
product lane. It is gated separately and must not be confused with the server-side component path.

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
- Server-side WASM component builds (`cargo component build --target wasm32-wasip1`) are now clean: no HTTP deps

---

## 5. What is still missing before 1.0

### 5.1 A truly usable primary binary

This is a major blocker.
If the primary `antikythera` binary is not fully usable, the end-user story is not finished.

### 5.2 Native CI quality gates

The project still needs strong automated gates for:

- `cargo test --workspace`
- `cargo clippy --workspace`
- `cargo fmt --check`

Without these, regressions will continue to slip through.

### 5.3 One final config contract

There must be a single decided configuration path and model that acts as the canonical source of truth.

### 5.4 Final API contract

Before 1.0, it must be made clear which surfaces are truly supported:

- REST
- JSON-RPC
- Rust SDK (native)
- Server-side WASM component (wasm32-wasip1, primary WASM target)
- Native C FFI (cdylib)
- Browser WASM (wasm-bindgen, secondary / optional)

The gap is not that the list is unclear, but that the **server-side WASM component** lane specifically
is missing:
- A complete set of WIT-defined exports that map to all real agent operations (not just stubs)
- A verified build pipeline that produces a `.wasm` binary embeddable by a non-Rust host
- An example host implementation (e.g. Python or Go) calling the component through wasmtime FFI
- Integration tests that validate the full WIT import/export contract end-to-end

None of these should remain half-final if the project wants to ship 1.0 confidently.

### 5.5 Runtime resilience

The project still needs stronger production behavior around:

- retry and backoff for LLM calls
- timeout policies
- cancellation
- context-window management
- authentication for REST
- observability, metrics, and health checks

---

## 6. Missing features that are strongly needed and fit this MCP client

### 6.1 Streaming responses

This is essential.

The project still needs:

- token streaming from providers
- SSE or similar streaming output for REST clients
- intermediate agent event streaming

Without this, UX for TUI, web, and agent consumers will feel behind modern expectations.

### 6.2 Context window management

This is critical for a real client framework.

The project needs:

- token estimation
- history pruning
- rolling summarization
- per-provider or per-model policy

Without it, long conversations will eventually fail at runtime.

### 6.3 Native provider-specific tool calling

The current agent flow still leans heavily on internal JSON conventions.

For a strong 1.0 client framework, it should support:

- OpenAI native tool calling
- Gemini tools
- Anthropic tools
- generic JSON fallback only where native support is unavailable

This would significantly improve reliability.

### 6.4 REST authentication and policy layer

If the REST server exposes `/chat` and `/tools`, it should have:

- bearer authentication
- per-route access policy
- rate limiting
- auditability

Without that, production deployment is risky.

### 6.5 Health, metrics, and observability endpoints

At minimum:

- `/health`
- `/metrics`
- correlation ID and session ID propagation
- structured telemetry

### 6.6 Transport plugin architecture

The current transport layer is still fairly fixed.
A strong framework would allow extension for:

- WebSocket transport
- custom internal bridges
- enterprise-specific transports

### 6.7 Real multi-agent orchestration, or remove it for now

If multi-agent remains public, it should become real.
If it is not ready, it is healthier to remove or hide it temporarily instead of presenting it as a supported feature.

---

## 7. What feels least correct architecturally right now

If the biggest mismatches are ranked:

1. **CLI duplicate universe**
2. **config fragmentation**
3. **public surface broader than implementation maturity**
4. **feature flags imply readiness that runtime does not always match**
5. **native/browser/component lanes are not yet strongly formalized**

---

## 8. Recommended implementation roadmap

## Phase 1 — Clean up the foundation

1. Make `antikythera-core` the single runtime source of truth.
2. Remove duplication from `antikythera-cli`; keep CLI as a thin adapter.
3. Unify the config system.
4. Decide which public features are truly supported.
5. Add full native CI gates.

## Phase 2 — Define product lanes clearly

Separate explicitly:

- **native** — full tokio async, HTTP providers, CLI binary
- **server-side WASM component** — `wasm32-wasip1` WASI, WIT imports/exports, hosted via wasmtime;
  host language calls the `.wasm` binary via FFI without needing a Rust runtime
- **native C FFI** — `cdylib` with `extern "C"` exports for embedding in Python/Go/C
- **browser WASM** — `wasm-bindgen`, `wasm32-unknown-unknown`, optional secondary target

Each lane should have:

- its own dependency expectations
- its own CI coverage
- its own documentation
- its own supported feature matrix

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

1. Make the CLI genuinely usable
2. Add full native CI
3. Finalize one config system
4. Remove duplicate architecture in the CLI
5. Finalize native / server-side WASM component / C FFI product lanes

### High priorities

6. Add streaming
7. Add context-window management
8. Add auth, metrics, and health
9. Add native provider-specific tool calling
10. Either complete or temporarily remove multi-agent as a public promise

---

## 10. Final conclusion

This repository has a strong foundation and a clear product direction, but it is still in the phase of an **ambitious pre-1.0 framework**, not yet a fully stable 1.0 framework.

The main issues are not lack of ideas. The main issues are:

- too many surfaces opened at once
- boundaries between crates are not strict enough
- some large features still exist more as architecture intent than final runtime implementation

If the project is cleaned up with discipline, it is well-positioned to become:

- a strong MCP client runtime
- a clean Rust / server-side WASM component / C FFI SDK family
- a production-ready agentic tool-using client framework

The right path forward is not to add many random features, but to:

**consolidate the architecture, narrow the public contract, and then add only the features that are truly required for the product.**
