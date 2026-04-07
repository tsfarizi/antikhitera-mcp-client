# Antikythera MCP Framework Documentation

Welcome to the comprehensive documentation for Antikythera MCP Framework.

## 📚 Documentation Index

### Core Documentation

| Document | Description |
|----------|-------------|
| [WASM Component with Host Imports](wasm-component-host-imports.md) | Architecture for WASM Component Model with host I/O delegation |
| [Server & Agent Management](server-agent-management.md) | Managing MCP Servers and Multi-Agents via WASM FFI |
| [FFI Reference](ffi.md) | Complete FFI API reference with examples in Python, Node.js, C#, Java |
| [CLI Documentation](CLI_DOCUMENTATION.md) | Complete CLI usage, commands, TUI interface, keyboard shortcuts |
| [Postcard Cache](POSTCARD_CACHE.md) | Binary configuration cache, performance benefits |
| [Config Import/Export](config-import-export.md) | Backup/restore config for infrastructure rebuilds |
| [JSON Schema Validation](json-schema-validation.md) | Enforce JSON output format with validation & auto-retry |
| [WASM Agent Architecture](wasm-agent-architecture.md) | Host-driven LLM calls, WASM processes responses |

### Build & Deployment

| Document | Description |
|----------|-------------|
| [Build Guide](BUILD.md) | How to build WASM components, CLI, and FFI libraries |
| [Testing Guide](TESTING_GUIDE.md) | How to run tests and verify builds |

### Migration & Legacy

| Document | Description |
|----------|-------------|
| [Migration Summary](MIGRATION_SUMMARY.md) | Changes from previous architecture |

## 🏗️ Architecture Overview

```
┌──────────────────────────────────────────────┐
│  Host Language (TypeScript/Python/Go)       │
│  ┌───────────────────────────────────────┐ │
│  │  MCP Server Management                │ │
│  │  ├─ Add/Remove servers                │ │
│  │  ├─ Start/Stop servers                │ │
│  │  └─ Export/Import configs             │ │
│  ├───────────────────────────────────────┤ │
│  │  Agent Management                     │ │
│  │  ├─ Register/Unregister agents        │ │
│  │  ├─ Execute tasks                     │ │
│  │  └─ Multi-agent orchestration         │ │
│  ├───────────────────────────────────────┤ │
│  │  Host Imports                         │ │
│  │  ├─ call-llm() → OpenAI/Anthropic     │ │
│  │  ├─ emit-tool-call() → MCP Servers    │ │
│  │  ├─ log-message() → Console           │ │
│  │  └─ save/load-state() → Database      │ │
│  └───────────────────────────────────────┘ │
│           ↑              ↓                 │
│     WASM Imports    WASM Exports           │
└───────────┼──────────────┼─────────────────┘
            │              │
┌───────────▼──────────────┼─────────────────┐
│  WASM Component (Rust)   │                 │
│  ┌─────────────────────┐ │                 │
│  │  Agent FSM Runner   │ │                 │
│  │  ├─ Parse LLM resp  │ │                 │
│  │  ├─ Detect actions  │ │                 │
│  │  ├─ Build prompts   │ │                 │
│  │  └─ Manage state    │ │                 │
│  └─────────────────────┘ │                 │
│  ┌─────────────────────┐ │                 │
│  │  Server Manager     │ │                 │
│  │  ├─ Registry        │ │                 │
│  │  ├─ Validation      │ │                 │
│  │  └─ FFI Interface   │ │                 │
│  ├─────────────────────┤ │                 │
│  │  Agent Manager      │ │                 │
│  │  ├─ Registry        │ │                 │
│  │  ├─ Orchestration   │ │                 │
│  │  └─ FFI Interface   │ │                 │
│  └─────────────────────┘ │                 │
└──────────────────────────┴─────────────────┘
```

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-component (for WASM builds)
cargo install cargo-component

# Install task runner
cargo install just
```

### Build WASM Component

```bash
# Generate WIT from Rust code
cargo run -p build-scripts --release -- wit

# Build WASM component
cargo component build --target wasm32-wasip1 --release

# Output: target/wasm32-wasip1/release/antikythera.wasm
```

### Build FFI Library

```bash
# Build shared library
cargo build --release --features ffi --lib

# Linux:   target/release/libantikythera_sdk.so
# Windows: target/release/antikythera_sdk.dll
# macOS:   target/release/libantikythera_sdk.dylib
```

### Run CLI

```bash
cargo run
```

## 📁 Project Structure

```
antikythera-mcp-framework/
├── documentation/          # ⭐ All documentation
│   ├── README.md           # This file
│   ├── wasm-component-host-imports.md
│   ├── server-agent-management.md
│   ├── ffi.md
│   ├── BUILD.md
│   ├── TESTING_GUIDE.md
│   └── MIGRATION_SUMMARY.md
│
├── antikythera-sdk/        # SDK (Vertical Slice Architecture)
│   ├── src/
│   │   ├── lib.rs          # Re-exports & documentation
│   │   ├── client/         # MCP Client (WASM bindings)
│   │   ├── prompts/        # Prompt Template Management
│   │   ├── servers/        # MCP Server Management
│   │   ├── agents/         # Multi-Agent Management
│   │   ├── response/       # Response Formatting
│   │   ├── config/         # Binary Configuration (Postcard)
│   │   └── component/      # WASM Component (Host Imports)
│   └── Cargo.toml
│
├── antikythera-core/       # Core MCP implementation
├── antikythera-cli/        # CLI application
└── scripts/                # Build scripts
    └── build-component.rs  # WIT generator
```

## 🧪 Running Tests

```bash
# Run all SDK tests
cargo test -p antikythera-sdk --test sdk_servers --test sdk_agents --test sdk_config

# Run specific test suite
cargo test -p antikythera-sdk --test sdk_servers
cargo test -p antikythera-sdk --test sdk_agents
cargo test -p antikythera-sdk --test sdk_config

# Run all workspace tests
cargo test --workspace
```

## 🔗 Useful Links

- [WASM Component Model Specification](https://github.com/WebAssembly/component-model)
- [cargo-component Documentation](https://github.com/bytecodealliance/cargo-component)
- [wit-bindgen Documentation](https://github.com/bytecodealliance/wit-bindgen)
- [WIT Interface Type Syntax](https://github.com/WebAssembly/component-model/blob/main/design/mdp/WIT.md)
