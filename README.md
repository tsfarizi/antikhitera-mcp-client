# Antikythera MCP Framework

Rust workspace for MCP client runtime, SDK bindings, session and logging support, plus WASM component tooling.

## Current state

This repository has a strong library and SDK surface, while the main end-user CLI surface is still partial.

| Surface | Current state |
|:--------|:--------------|
| `antikythera-core` | Main implementation for client, agent runtime, configuration, providers, and transports |
| `antikythera-sdk` | Richest public API: Rust wrapper, WASM bindings, FFI helpers, config, session, server, and agent utilities |
| `antikythera-session` | Session and history management with export/import support |
| `antikythera-log` | Structured logging and subscription utilities |
| `antikythera` binary | Builds, but `tui` and `rest` modes still return placeholder output |
| `antikythera-config` binary | Usable CLI flow for lightweight Postcard-based config management |

## Quick start

### Prerequisites

- Rust 1.75+
- `cargo-component` for WASM component builds
- Optional: `task` for `Taskfile.yml` helpers

### Build the workspace

```bash
cargo build --workspace
```

### Run the current binaries

```bash
# Main CLI entry point
cargo run -p antikythera-cli --bin antikythera

# Explicit mode selection
cargo run -p antikythera-cli --bin antikythera -- --mode tui
cargo run -p antikythera-cli --bin antikythera -- --mode rest

# Config CLI
cargo run -p antikythera-cli --bin antikythera-config -- --help
```

### Build the component

```bash
# Generate WIT from Rust source
cargo run -p build-scripts --release -- wit

# Build the component
cargo component build -p antikythera-sdk --release --target wasm32-wasip1
```

### Build the docs site

```bash
mdbook build
```

## Workspace

| Path | Purpose |
|:-----|:--------|
| `antikythera-core/` | Core MCP client, agent runtime, config, infrastructure integrations |
| `antikythera-sdk/` | Rust API, WASM bindings, FFI/config/session/server helpers |
| `antikythera-cli/` | Native binaries: `antikythera` and `antikythera-config` |
| `antikythera-session/` | Session state and persistence |
| `antikythera-log/` | Structured logging |
| `tests/` | Workspace integration and crate-level test suites |
| `scripts/` | WIT and component build tooling |
| `wit/` | Generated WIT definitions |
| `documentation/` | Focused documentation files with direct uppercase names |

## Documentation

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
| [`documentation/FFI.md`](documentation/FFI.md) | FFI surface and integration notes |
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

- Workspace version is `0.9.5`.
- The current documentation names under `documentation/` use uppercase direct filenames for consistency.
- The main CLI binary is still partial, so the docs now distinguish clearly between implemented behavior and planned/runtime placeholder behavior.
