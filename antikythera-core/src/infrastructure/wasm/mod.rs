//! Wasmtime-based WASM agent runner.
//!
//! # Module ABI
//!
//! A WASM module that wants to run as a sandboxed agent must expose this ABI:
//!
//! ## Exports (module → host)
//!
//! | Export | Signature | Description |
//! |---|---|---|
//! | `memory` | linear memory | The module's linear memory. |
//! | `antikythera_alloc` | `(len: i32) -> i32` | Allocate `len` bytes; returns ptr. |
//! | `antikythera_dealloc` | `(ptr: i32, len: i32)` | Free a previously allocated region. |
//! | `antikythera_run` | `(task_ptr: i32, task_len: i32) -> i64` | Run a task. Input is the task JSON at `task_ptr[..task_len]`. Returns `(result_ptr << 32) | result_len`. Returns a negative value on error. |
//!
//! ## Imports (host → module)
//!
//! | Import | Signature | Description |
//! |---|---|---|
//! | `antikythera::call_llm_sync` | `(req_ptr: i32, req_len: i32) -> i64` | Synchronously call the LLM with a request JSON at `req_ptr[..req_len]`. Returns `(result_ptr << 32) | result_len`, or negative on error. The returned memory is owned by the host and written into WASM memory via `antikythera_alloc`. |
//!
//! # Threading
//!
//! The WASM execution is entirely synchronous from the host's perspective.
//! `WasmAgentRunner::run_task` runs the WASM module inside a
//! `tokio::task::spawn_blocking` call, which allows the calling async task to
//! remain unblocked while the WASM guest executes.
//!
//! Because the LLM `call_llm_sync` import must block inside the wasmtime host
//! function, it uses `tokio::task::block_in_place` internally.  This requires
//! a **multi-thread** tokio runtime; the `wasm-runtime` feature therefore
//! documents this requirement.

#[cfg(feature = "wasm-runtime")]
pub use runner::WasmAgentRunner;
#[cfg(feature = "wasm-runtime")]
pub use runner::WasmRuntimeError;

#[cfg(feature = "wasm-runtime")]
mod runner;
