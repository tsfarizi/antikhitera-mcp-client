# antikythera-sdk

High-level SDK and WASM component surface for the Antikythera MCP Framework.

## Features

- FFI-ready API with C-string helpers and unified `ffi_handler!` macro
- WASM agent runner with session lifecycle, LLM response processing, tool validation
- Prompt management FFI bindings
- Postcard-based configuration serialization
- SDK logging with per-module loggers and query API
- Session and log re-exports for host integration

## Feature Flags

- `sdk-core` — re-exports core types (`Agent`, `McpClient`, `AppConfig`, etc.)
- `component` — WASM agent types, processor, and runner
- `multi-agent` — multi-agent orchestration SDK
- `single-agent` — single-agent operations
- `wasm-sandbox` — WASM sandboxing support
- `full` — enables all features
