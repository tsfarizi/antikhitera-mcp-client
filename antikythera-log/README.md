# antikythera-log

Structured logging foundation for the Antikythera MCP Framework.

## Features

- Thread-safe ring-buffer logger (`Logger`, `LogBuffer`)
- 16 convenience macros: `alog_debug!`, `alog_info!`, `alog_warn!`, `alog_error!`, plus source/context variants, `cli_print!`, `cli_eprint!`
- Filtering, pagination, JSON export, batch/session log export
- Real-time log subscription (`LogSubscriber`)
- Postcard binary serialization (`PostcardSerde` trait)
- Compile-time lint to ban raw `println!`/`dbg!`/`tracing!` usage (feature: `lint`)

## Feature Flags

- `wasm` — wasm-bindgen support
- `subscriber` — tokio + crossbeam-channel for real-time log streaming
- `lint` — shadows println!/eprintln!/dbg!/tracing macros with compile errors
