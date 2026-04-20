# Antikythera MCP Framework v0.9.7

Rust MCP client framework with two focused deployment lanes: **native CLI** and **server-side WASM component**.

The framework handles agent orchestration, tool calling, session management, and context window control—while delegating **all LLM API invocation to the embedding host** for security, flexibility, and cost control.

## Deployment Lanes

This framework is designed as a two-lane runtime targeting different deployment contexts:

### 1. **Native CLI Lane** ✅ ACTIVE

Full tokio async runtime, native stdio TUI, multi-agent orchestration.

- **Build target:** x86_64, ARM64, or any native platform
- **Runtime:** Full tokio with `std` I/O
- **Provider support:** HTTP-based LLM providers (Gemini, OpenAI, Ollama via `DynamicModelProvider`)
- **Binaries:**
  - `antikythera` — Interactive TUI for agent interaction
  - `antikythera-config` — Configuration wizard and management
- **Features:** Multi-agent orchestration, streaming responses, context window management with rolling summarization
- **Use case:** Developers, interactive testing, local/on-prem deployments

**Quick start (native):**
```bash
cargo build --release -p antikythera-cli
./target/release/antikythera --mode stdio
# or
./target/release/antikythera --mode multi-agent --agents agents.json --task "Your task here"
```

### 2. **Server-Side WASM Component Lane** ✅ PRIMARY

Portable WASI component (wasm32-wasip1) hosted by any language (Python, Go, TypeScript, etc.) via wasmtime.

- **Build target:** `wasm32-wasip1` (WASI component model)
- **Runtime:** Sandboxed wasmtime with zero std I/O
- **Provider support:** Host-provided via WIT imports (`call_llm_sync`, `call_tool_sync`)
- **Interface:** WIT (WebAssembly Interface Types) with 6 multi-agent + 6 resilience functions
- **Features:** Agent state isolation, session history management, context policies, tool result processing
- **Use case:** Polyglot AI services, enterprise embeddings, multi-tenant platforms, language-agnostic AI agents
- **Host owns:** LLM API calls, tool execution, auth/policy enforcement, observability/telemetry

**Quick start (WASM component):**
```bash
cargo component build -p antikythera-sdk --release --target wasm32-wasip1 \
  --no-default-features --features component
# Produces: target/wasm32-wasip1/release/antikythera_sdk.wasm
```

Host integration example (Python):
```python
from wasmtime import Instance, Module, Store, Linker

# Load and run WASM component
module = Module.from_file(store, "antikythera_sdk.wasm")
instance = Instance(store, module, linker)

# Call WASM exports: init, prepare_user_turn, commit_llm_response, get_state
session_id = instance.exports(store).init(config_json)
prepared = instance.exports(store).prepare_user_turn(request_json)
result = instance.exports(store).commit_llm_response(prepared, llm_response_json)
```

---

## Surface Maturity

| Surface | Status |
|:--------|:--------|
| `antikythera-core` | ✅ Mature — Main MCP client, agent runtime, config, transports, resilience |
| `antikythera-sdk` | ✅ Stable — Rust API, native lane + WASM component lane, session/logging |
| `antikythera-cli` | ✅ Active — `stdio` (interactive TUI), `setup` (config wizard), `multi-agent` (orchestrator) |
| `antikythera-session` | ✅ Solid — Session and history management with postcard serialization |
| `antikythera-log` | ✅ Solid — Structured logging and subscription |
| **Browser WASM** | ❌ Removed — Use native lane or server-side WASM component instead |
| **C FFI** | ❌ Removed — Embedding hosts provide their own bindings |
| **REST API** | ❌ Removed — Embedding hosts own the interface layer |

## Prerequisites & Building

### Requirements

- Rust 1.75+
- `cargo-component` (for WASM builds: `cargo install cargo-component`)
- Optional: `task` command runner for helpers

### Native Lane: CLI Build & Run

```bash
# Build all CLI binaries
cargo build --release -p antikythera-cli

# Run interactive TUI
./target/release/antikythera --mode stdio

# Setup wizard
./target/release/antikythera --mode setup

# Multi-agent orchestrator
./target/release/antikythera --mode multi-agent \
  --agents agents.json \
  --task "Analyze this data" \
  --execution-mode parallel:4
```

### WASM Component Lane: Build & Embed

```bash
# Build the WASM component for server-side embedding
cargo component build -p antikythera-sdk --release --target wasm32-wasip1 \
  --no-default-features --features component

# Output: target/wasm32-wasip1/release/antikythera_sdk.wasm
# Load this into any wasmtime host (Python, Go, Rust, Node.js, etc.)
```

### Test & Lint

```bash
# Run all tests
cargo test --workspace

# Lint (warnings treated as errors)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --all -- --check

# Or use helpers:
task test
task lint
task fmt
```

---

## Host Integration Contract (WASM Component)

The server-side WASM component implements a **host-driven model** where the framework handles state and logic, while the host owns interface layers and LLM calls.

### Two-Phase Chat Flow

**Phase 1: Prepare** — Framework builds session-aware message context
```rust
prepare_user_turn(request_json) -> prepared_turn_json
// input: { "prompt": "...", "session_id": "...", "system_prompt": "...", "context_policy": {...} }
// output: { "messages": [...], "context": {...}, "turn_id": "..." }
```

**Phase 2: Commit** — Host invokes LLM, then commits response back into framework session
```rust
commit_llm_response(prepared_turn_json, llm_response_json) -> agent_action_json
// Host calls LLM with prepared messages
// Framework normalizes response and updates session state
// Returns: { "action": "final"|"call_tool"|"retry", "content": "...", "session_id": "..." }
```

### Host Responsibilities

- ✅ LLM API calls (OpenAI, Anthropic, Gemini, etc.) — framework provides structured prompt only
- ✅ Tool execution (command, API call, search) — framework provides tool spec only
- ✅ Auth/policy enforcement (who can call which agent, which tools)
- ✅ Observability/telemetry (logging, tracing, metrics)
- ✅ Persistence (session storage, audit logs)

### Framework Responsibilities

- ✅ Session state management (conversation history, agent step tracking)
- ✅ Message normalization (accept plain text or structured JSON actions)
- ✅ Tool result processing (validate, integrate into history)
- ✅ Context window management (token estimation, rolling summarization, truncation)
- ✅ Retry logic and resilience policies (backoff, timeouts)
- ✅ Multi-agent orchestration (router, scheduler, execution modes)

### Runtime hardening and monitoring APIs

For multi-agent hosts, SDK-level runtime controls are available to adjust
hardening behavior and inspect live execution state without rebuilding:

- `configure_hardening(options_json)`
- `cancel_orchestrator()`
- `get_monitor_snapshot()`
- `task_result_detail(task_result_json)`

WASM session lifecycle hardening APIs are also available for host persistence:

- `sweep_idle_sessions(now_unix_ms)`
- `hydrate_session(session_id, state_json)`
- `report_session_restore_progress(session_id, progress_json)`

Actionable observability is exposed via:

- `get_slo_snapshot(session_id)`

SLO payload includes:

- `success_rate`
- `tool_error_rate`
- `retry_ratio`
- `p95_prepare_latency_ms`
- `p95_commit_latency_ms`

Correlation ID is propagated end-to-end from `prepare_user_turn` through
`commit_llm_response` and `process_tool_result` event emission so host logs and
user-facing feedback streams can be traced consistently during incidents.

When in-memory capacity is exceeded (`max_in_memory_sessions`) or timeout policy
marks a session as inactive, the runner emits stream events that hosts can
forward to telemetry/UI:

- `session_archived` (includes `state_json` snapshot for host persistence)
- `session_restore_requested`
- `session_restore_progress`
- `session_restored`

The native CLI path also enables provider stream chunk events and installs a
terminal sink (stderr) so streamed content is visible live while stdout stays
safe for machine-readable output.

---

---

## Workspace Structure

| Path | Purpose |
|:-----|:--------|
| `antikythera-core/` | Core MCP client, agent runtime, config, resilience, multi-agent orchestrator |
| `antikythera-sdk/` | Rust API surface: native lane helpers + WASM component lane exports |
| `antikythera-cli/` | Native binaries: `antikythera` (TUI/orchestrator) and `antikythera-config` |
| `antikythera-session/` | Session state and postcard persistence |
| `antikythera-log/` | Structured logging and subscription |
| `tests/` | Workspace integration tests and crate-level suites |
| `wit/` | WASM Interface Types definitions for component model |
| `documentation/` | Focused reference guides (all uppercase filenames) |

This repository now keeps **one README only** at the repository root. Detailed explanations live in dedicated files under `documentation/`.

### Visual guides

These are the best starting points if you want diagram-based explanations.

| Document | Focus |
|:---------|:------|
| [`documentation/WORKSPACE.md`](documentation/WORKSPACE.md) | Workspace map and crate responsibilities |
| [`documentation/ARCHITECTURE.md`](documentation/ARCHITECTURE.md) | High-level runtime architecture and data flow |
| [`documentation/CLI.md`](documentation/CLI.md) | Current CLI binaries, their limits, and config flow |
| [`documentation/BUILD.md`](documentation/BUILD.md) | Build, test, lint, and component flow |
| [`documentation/COMPONENT.md`](documentation/COMPONENT.md) | WASM component model and host-import interaction |

The static docs site is generated from this `README.md`, `SUMMARY.md`, and the markdown files under `documentation/` via `mdBook`.

### Reference guides

| Document | Focus |
|:---------|:------|
| [`documentation/CONFIG.md`](documentation/CONFIG.md) | Postcard-based configuration details |
| [`documentation/CACHE.md`](documentation/CACHE.md) | Configuration cache behavior |
| [`documentation/IMPORT_EXPORT.md`](documentation/IMPORT_EXPORT.md) | Config import/export workflow |
| [`documentation/JSON_SCHEMA.md`](documentation/JSON_SCHEMA.md) | JSON schema validation and retry flow |
| [`documentation/LOGGING.md`](documentation/LOGGING.md) | Logging model and usage |
| [`documentation/SERVERS_AND_AGENTS.md`](documentation/SERVERS_AND_AGENTS.md) | Server and agent management surface |
| [`documentation/WASM_AGENT.md`](documentation/WASM_AGENT.md) | WASM-side agent behavior |
| [`documentation/TESTING.md`](documentation/TESTING.md) | Test commands and test categories |
| [`documentation/MIGRATION.md`](documentation/MIGRATION.md) | Historical migration notes |

## Core development commands

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace

# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings
```

If you use Task, the repository also exposes helpers such as `task build`, `task build-cli`, `task test`, `task lint`, and `task wit`.

## Notes

- Workspace version is `0.9.7`.
- The current documentation names under `documentation/` use uppercase direct filenames for consistency.
- For WASM component embeddings, start with [`documentation/COMPONENT.md`](documentation/COMPONENT.md).
- For native CLI, start with [`documentation/CLI.md`](documentation/CLI.md).
