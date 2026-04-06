# 🤖 AGENTS.md - AI Coding Agent Guide

> **Purpose**: Quick reference for AI coding agents to understand this codebase.

---

## 📍 Repository Overview

**What is this?** A Rust-based MCP (Model Context Protocol) client with:
- Full TUI interface (Ratatui)
- Multi-provider LLM support (Gemini, OpenAI, Ollama, Anthropic)
- Agent mode with tool calling
- REST API server

**Tech Stack**: Rust 1.75+, Tokio (async), Ratatui (TUI), Axum (HTTP), Serde (JSON)

---

## 🗂️ File Map

```
src/
├── bin/                    # Entry points
│   ├── menu.rs            # Main TUI entry (cargo run)
│   ├── rest.rs            # REST-only mode (uses config bind)
│   └── stdio.rs           # STDIO-only mode
│
└── lib/                    # Main library
    ├── mod.rs             # Top-level orchestration, run() function
    │
    ├── application/       # Business logic layer
    │   ├── agent/         # Agent with tool-calling loop
    │   │   ├── runner.rs  # Core ReAct loop (thought → action → observe)
    │   │   ├── parser.rs  # JSON response parsing
    │   │   └── prompt.rs  # System prompt composition
    │   ├── client.rs      # McpClient - main client facade
    │   ├── stdio.rs       # Legacy text-based STDIO (deprecated)
    │   └── tooling.rs     # Tool execution runtime
    │
    ├── cli/               # CLI argument parsing (clap)
    │
    ├── config/            # Configuration loading & wizard
    │   ├── app.rs         # AppConfig, RestServerConfig structs
    │   ├── loader.rs      # Loads BOTH client.toml + model.toml
    │   ├── wizard/        # Interactive setup wizard
    │   │   ├── mod.rs     # Wizard orchestration
    │   │   └── generators/    # ⭐ SPLIT GENERATORS
    │   │       ├── mod.rs
    │   │       ├── client.rs  # Writes to client.toml
    │   │       └── model.rs   # Writes to model.toml
    │   └── generator.rs   # Legacy (kept for compatibility)
    │
    ├── constants.rs       # CONFIG_PATH, MODEL_CONFIG_PATH, etc.
    │
    ├── domain/            # Domain types (tool definitions, etc.)
    │
    ├── infrastructure/    # External integrations
    │   ├── model/         # LLM provider implementations
    │   │   ├── gemini.rs
    │   │   ├── openai.rs
    │   │   ├── ollama.rs
    │   │   └── anthropic.rs
    │   ├── server/        # REST API server (Axum)
    │   └── rpc/           # JSON-RPC for MCP communication
    │
    └── tui/               # TUI layer (Ratatui)
        ├── terminal.rs    # Terminal init/restore, NavAction enum
        ├── widgets/       # Reusable widgets (Menu, TableMenu)
        └── screens/       # Screen modules
            ├── mode_selector.rs   # Mode selection menu
            ├── setup_menu/        # Configuration management
            │   ├── mod.rs         # Main setup menu
            │   ├── providers.rs   # Manage LLM providers
            │   ├── models.rs      # Manage models
            │   ├── servers.rs     # Manage MCP servers
            │   ├── sync.rs        # Sync tools from servers
            │   └── prompt.rs      # Manage Prompt Template
            └── chat/              # Interactive chat (SOLID)
                ├── state.rs       # ChatState, ChatMessage
                ├── ui.rs          # Rendering (status, messages, input)
                ├── input.rs       # Keyboard handling
                └── runner.rs      # Async event loop

config/                         # ⭐ SPLIT CONFIGURATION
├── client.toml                # Infrastructure: providers, servers, REST
├── model.toml                 # Model: default_provider, model, prompt, tools
└── .env                       # API keys (gitignored)

wit/                           # ⭐ AUTO-GENERATED WIT INTERFACES
└── antikythera.wit           # Generated from Rust traits (don't edit!)

antikythera-sdk/               # SDK & WASM bindings
├── src/
│   ├── lib.rs                # Main SDK entry point
│   ├── component.rs          # ⭐ Traits for WIT generation (source of truth)
│   ├── wasm.rs               # wasm-bindgen bindings (browser)
│   ├── wasm_prompt.rs        # Prompt WASM functions (browser)
│   └── high_level_api.rs     # Native Rust API
└── Cargo.toml                # SDK configuration

scripts/                       # ⭐ BUILD SCRIPTS (Rust idiomatic)
├── build-component.rs        # WIT generator + component builder
└── Cargo.toml                # Build script package
```

---

## 🔑 Key Patterns

### 1. Terminal Reference Passing
TUI screens accept `&mut Tui` parameter to avoid reinitializing terminal:
```rust
pub fn run_manage_providers_with_terminal(terminal: &mut Tui) -> Result<()>
```

### 2. NavAction Enum
Single enum for all navigation actions in `tui/terminal.rs`:
```rust
pub enum NavAction {
    Up, Down, Select, Back, ForceQuit, None
}
```
- `q` key → `ForceQuit` (exits entire app)
- `ESC` → `Back` (go back one screen)

### 3. Async Channel Pattern (Chat)
Chat uses `mpsc::channel` for async LLM responses:
```rust
let (response_tx, mut response_rx) = mpsc::channel::<ResponseEvent>(10);
tokio::spawn(async move { send_message(..., tx).await; });
// In loop: response_rx.try_recv()
```

### 4. Split Configuration Loading
```rust
// Loader reads from BOTH files
let config = AppConfig::load(Some(Path::new("config/client.toml")))?;
// Internally derives model.toml from same directory
```

**Config files:**
- `config/client.toml` → providers, servers, `[server]` (bind, cors)
- `config/model.toml` → default_provider, model, prompt_template, tools
- `config/.env` → API key values

### 5. Split Generators
```rust
// For model.toml (default_provider, model, prompt_template, tools)
use crate::config::wizard::generators::model;
model::update_default_provider("gemini")?;
model::sync_tools_from_server("time", tools)?;

// For client.toml (providers, servers, REST settings)
use crate::config::wizard::generators::client;
client::add_provider("ollama", "ollama", "http://localhost:11434", None)?;
client::add_server("time", "/path/to/server", &[])?;
```

### 6. Agent ReAct Loop
Agent expects JSON responses from LLM:
```json
{"action": "call_tool", "tool": "tool_name", "arguments": {...}}
{"action": "final", "response": "Final answer text"}
```

---

## 🧭 Common Tasks

### Add a new TUI screen
1. Create `src/lib/tui/screens/your_screen.rs`
2. Function signature: `pub fn run_your_screen(terminal: &mut Tui) -> Result<()>`
3. Export in `screens/mod.rs`

### Add a new LLM provider
1. Implement `ModelProvider` trait in `infrastructure/model/`
2. Add to `DynamicModelProvider::from_configs()`

### Add a new command in chat
1. Add variant to `CommandResult` in `chat/input.rs`
2. Add to `parse_command()` match
3. Handle in `handle_command()` in `chat/runner.rs`

### Modify config structure
1. Update structs in `config/app.rs` (RestServerConfig, AppConfig)
2. Update loader in `config/loader.rs`
3. Update generators:
   - `config/wizard/generators/client.rs` for client.toml changes
   - `config/wizard/generators/model.rs` for model.toml changes
4. Update `config.example/client.toml` and `config.example/model.toml`

### Add REST server configuration
1. Add field to `RestServerConfig` in `config/app.rs`
2. Update CLI args in `cli/mod.rs` and `bin/rest.rs`
3. Update `lib/mod.rs` to use config as default

---

## ⚠️ Gotchas

1. **PowerShell exit codes**: `cargo build` may show exit code 1 due to PowerShell parsing, check for "Finished" text instead.

2. **Terminal state**: Always restore terminal on error paths to prevent broken terminal state.

3. **Unused code warnings**: Legacy functions may exist for backwards compatibility, check `#[allow(dead_code)]`.

4. **Async in TUI**: Use `tokio::task::block_in_place` or channels to bridge sync TUI with async operations.

5. **Windows line endings**: Files may have CRLF endings.

6. **Split config files**: `client.toml` and `model.toml` must both exist. Loader derives `model.toml` path from `client.toml`'s parent directory.

7. **REST bind address**: Configured in `[server].bind` in `client.toml`. CLI `--addr` flag overrides it.

8. **Generator separation**: Use `generators::model` for model.toml updates, `generators::client` for client.toml updates.

---

## 🔧 Quick Commands

```bash
cargo run                    # TUI mode selector
cargo run --bin stdio        # Direct STDIO mode
cargo run --bin mcp-rest     # Direct REST mode (uses config bind)
cargo run --bin mcp-rest -- --addr 0.0.0.0:3000  # Override bind
cargo run -- --mode rest     # REST via main binary
cargo run -- --mode all      # STDIO + REST together
cargo build                  # Build debug
cargo build --release        # Build production
cargo clippy                 # Lint
cargo fmt                    # Format

# WASM Component (Automatic WIT Generation)
cargo run --release -p build-scripts -- wit       # Generate WIT from Rust → wit/antikythera.wit
cargo run --release -p build-scripts -- component # Build WASM component
cargo run --release -p build-scripts -- all       # Generate + Build
wasm-tools component wit *.wasm                   # View generated WIT
```

---

## 🧩 WASM Component Model (Automatic WIT Generation)

The SDK uses **witgen** to automatically generate WIT interfaces from Rust code.

### How It Works

```
Rust code + #[witgen]  →  witgen  →  WIT auto-generated  →  cargo-component  →  *.wasm
```

**No manual WIT editing needed!** Add functions to Rust traits and WIT is regenerated automatically.

### Adding New Component Functions

1. **Add to trait** in `antikythera-sdk/src/component.rs`:
```rust
use witgen::witgen;

#[witgen]
pub trait MyInterface {
    /// My new function
    fn my_function(param: String) -> Result<String, String>;
}
```

2. **Build**:
```bash
cargo run --release -p build-scripts -- all
```

3. **Done!** WIT is auto-generated at `wit/antikythera.wit` and component is built.

### Key Rules for `#[witgen]`

- ✅ Use `String` (not `&str`)
- ✅ Use `Vec<T>` for lists
- ✅ Use `Option<T>` for optional values
- ✅ Use `Result<T, String>` for errors
- ❌ No references (`&T`, `&mut T`)
- ❌ No smart pointers (`Box`, `Rc`, `Arc`)
- ❌ No generics

### Build Script Location

Build scripts are in `scripts/` directory (idiomatic Rust, not bash):
- `scripts/build-component.rs` - Main build script
- Run with: `cargo run --release -p build-scripts -- <command>`

### WIT File Location

Generated WIT file is at project root for easy visibility:
- `wit/antikythera.wit` - Auto-generated from Rust traits
- Don't edit this file manually!
- Edit `antikythera-sdk/src/component.rs` instead

### Workflow

```
1. Edit traits in antikythera-sdk/src/component.rs
2. Run: cargo run --release -p build-scripts -- wit
3. WIT auto-generated at: wit/antikythera.wit
4. Build: cargo run --release -p build-scripts -- component
```

---

## 📊 Agent Architecture

```mermaid
graph TD
    User([👤 User]) <-->|Input/Output| TUI[🖥️ TUI Chat]
    
    TUI --> Agent[🤖 Agent]
    
    subgraph "Agent Brain"
        Context[📝 Context & History]
        Planner[🧠 Planner]
    end
    
    subgraph "Capabilities"
        LLM[☁️ LLM Provider]
        Tools[🛠️ MCP Tools]
    end
    
    Agent --> Context
    Agent --> Planner
    Planner <--> LLM
    Planner <-->|Call| Tools
    
    style TUI fill:#6c5ce7,stroke:#fff,color:#fff
    style Agent fill:#1a1a2e,stroke:#fff,color:#fff
    style LLM fill:#0f3460,stroke:#fff,color:#fff
    style Tools fill:#533483,stroke:#fff,color:#fff
```

---

## 📝 Code Style

- **Error handling**: Use `Box<dyn Error>` for flexibility, `thiserror` for custom errors
- **Logging**: Use `tracing` crate (`info!`, `debug!`, `error!`)
- **Async**: Tokio runtime, avoid blocking in async contexts
- **TUI**: Ratatui with crossterm backend
- **Config**: TOML format with serde

---

## 🔗 Key Dependencies

| Crate | Purpose |
|:------|:--------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal manipulation |
| `tokio` | Async runtime |
| `axum` | HTTP server |
| `serde` | Serialization |
| `clap` | CLI parsing |
| `tracing` | Logging |
| `wasm-bindgen` | WASM browser bindings |
| `wit-bindgen-rt` | WASM Component Model bindings |
