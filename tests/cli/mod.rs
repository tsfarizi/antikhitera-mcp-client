//! CLI Test Suite — centralized in tests/cli/
//!
//! Entry point for entire CLI unit test suite, moved from `antikythera-cli/tests/`.
//! Each module corresponds to one source module.
//!
//! Run:
//!   cargo test -p antikythera-tests --test cli_tests

#[path = "error.rs"]
mod error;

#[path = "cli.rs"]
mod cli;

#[path = "config.rs"]
mod config;

#[path = "entities.rs"]
mod entities;

#[path = "infrastructure_config.rs"]
mod infrastructure_config;

#[path = "chat_use_case.rs"]
mod chat_use_case;

#[path = "runtime.rs"]
mod runtime;

#[path = "wasm_harness.rs"]
mod wasm_harness;

#[path = "scenario.rs"]
mod scenario;

#[path = "commands_tests.rs"]
mod commands_tests;

#[path = "log_panel_tests.rs"]
mod log_panel_tests;

#[path = "stdio_tests.rs"]
mod stdio_tests;
