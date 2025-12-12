<div align="center">

# 🚀 MCP Client

**A flexible Model Context Protocol client with modern TUI interface**

[![Rust](https://img.shields.io/badge/rust-v1.75%2B-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](http://makeapullrequest.com)

[Features](#-features) •
[Quick Start](#-quick-start) •
[TUI Interface](#-tui-interface) •
[Configuration](#%EF%B8%8F-configuration) •
[API](#-api-reference) •
[Development](#-development)

</div>

---

## ✨ Features

| Feature | Description |
|:--------|:------------|
| 🖥️ **Modern TUI** | Full Ratatui-based interface with responsive layout |
| 🤖 **Multi-Provider** | Gemini, Ollama, OpenAI, Anthropic, and more |
| 🔧 **MCP Tools** | Connect any MCP-compatible tool server |
| 💬 **Interactive Chat** | Real-time chat with Agent/Chat mode toggle |
| 🌐 **REST API** | HTTP API with Swagger UI |
| ⚡ **Dual Mode** | Run STDIO + REST simultaneously |
| 🔄 **Hot Reload** | Reload config without restart |
| 📝 **Setup Wizard** | TUI-based configuration management |

---

## 🏗️ Architecture

```mermaid
graph TB
    subgraph TUI["🖥️ TUI Layer (Ratatui)"]
        MENU["Mode Selector"]
        CHAT["💬 Chat Interface"]
        SETUP["⚙️ Setup Menu"]
    end
    
    subgraph Client["🔧 MCP Client Core"]
        CLI["CLI Parser"]
        APP["Application Layer"]
        
        subgraph Core["Core Components"]
            AGENT["🤖 Agent"]
            MCPC["MCP Client"]
            TOOL["🔧 Tool Manager"]
        end
    end
    
    subgraph Providers["☁️ LLM Providers"]
        GEMINI["Gemini"]
        OLLAMA["Ollama"]
        OPENAI["OpenAI"]
        ANTHROPIC["Anthropic"]
    end
    
    subgraph Servers["🔌 MCP Servers"]
        SRV1["Tool Server 1"]
        SRV2["Tool Server 2"]
        SRVN["Tool Server N"]
    end
    
    MENU --> CHAT
    MENU --> SETUP
    MENU --> REST["🌐 REST API"]
    CHAT --> MCPC
    MCPC --> AGENT
    AGENT --> TOOL
    MCPC --> Providers
    TOOL --> Servers
    
    style TUI fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style Client fill:#1a1a2e,stroke:#16213e,color:#fff
    style Providers fill:#0f3460,stroke:#16213e,color:#fff
    style Servers fill:#533483,stroke:#16213e,color:#fff
```

---

## 🚀 Quick Start

### Prerequisites

| Requirement | Version | Note |
|:------------|:--------|:-----|
| Rust | 1.75+ | Edition 2024 |
| Ollama | Latest | Optional, for local models |
| API Key | - | For cloud providers |

### Installation

```bash
# Clone repository
git clone https://github.com/your-username/antikhitera-mcp-client.git
cd antikhitera-mcp-client

# Build
cargo build --release

# Run (shows TUI mode selector)
cargo run
```

---

## 🖥️ TUI Interface

### Mode Selection

```mermaid
flowchart LR
    A["cargo run"] --> B{{"🚀 Mode Selector"}}
    B -->|1| C["💬 STDIO Chat"]
    B -->|2| D["🌐 REST API"]
    B -->|3| E["⚡ Both"]
    B -->|4| F["⚙️ Setup"]
    
    style A fill:#2d3436,stroke:#636e72,color:#fff
    style B fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style C fill:#00b894,stroke:#00cec9,color:#fff
    style D fill:#0984e3,stroke:#74b9ff,color:#fff
    style E fill:#fd79a8,stroke:#e84393,color:#fff
    style F fill:#fdcb6e,stroke:#f39c12,color:#000
```

### Chat Interface Layout

```mermaid
flowchart TB
    subgraph UI["🖥️ Chat Interface"]
        direction TB
        
        subgraph status["Status Bar"]
            s1["Session ID"]
            s2["Mode: Agent/Chat"]
            s3["Provider/Model"]
        end
        
        subgraph msgs["Message Area"]
            m1["👤 User messages"]
            m2["🤖 AI responses"]
            m3["� Scrollable history"]
        end
        
        subgraph input["Input Box"]
            i1["> Type message here..."]
        end
        
        subgraph help["Help Bar"]
            h1["Keybinding hints"]
        end
    end
    
    status --> msgs
    msgs --> input
    input --> help
    
    style UI fill:#1a1a2e,stroke:#6c5ce7,color:#fff
    style status fill:#2d3436,stroke:#636e72,color:#fff
    style msgs fill:#0f3460,stroke:#74b9ff,color:#fff
    style input fill:#0984e3,stroke:#74b9ff,color:#fff
    style help fill:#2d3436,stroke:#636e72,color:#fff
```

#### Layout Components

| Component | Description |
|:----------|:------------|
| **Status Bar** | Session ID, Agent/Chat mode toggle, Provider & Model info |
| **Message Area** | Scrollable chat history with user and AI messages |
| **Input Box** | Text input with cursor, shows "Command" when typing `/` |
| **Help Bar** | Context-sensitive keybinding hints |


### Chat Keybindings

| Key | Action |
|:----|:-------|
| `Enter` | Send message |
| `q` | Exit (when input empty) |
| `Ctrl+C` | Clear input |
| `PageUp/Down` | Scroll messages |
| `/help` | Show commands |
| `/agent on\|off` | Toggle agent mode |
| `/reset` | Reset session |
| `/logs` | Show last logs |
| `/steps` | Show tool steps |

### Setup Menu

```mermaid
flowchart TD
    SETUP["⚙️ Setup Menu"] --> PROV["Manage Providers"]
    SETUP --> MODEL["Manage Models"]
    SETUP --> SRV["Manage MCP Servers"]
    SETUP --> PROMPT["Manage Prompt Template"]
    
    PROV --> ADD_P["Add Provider"]
    PROV --> VIEW_P["View/Remove"]
    
    SRV --> ADD_S["Add Server"]
    SRV --> SYNC["Sync Tools"]
    SRV --> DETAIL["View Details"]
    
    PROMPT --> EDIT["Direct Edit"]
    PROMPT --> VIEW_T["View Template"]
    PROMPT --> RESET["Reset Default"]
    
    style SETUP fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style PROV fill:#00b894,stroke:#00cec9,color:#fff
    style MODEL fill:#0984e3,stroke:#74b9ff,color:#fff
    style SRV fill:#fd79a8,stroke:#e84393,color:#fff
    style PROMPT fill:#fdcb6e,stroke:#f39c12,color:#000
```

---

## ⚙️ Configuration

### Directory Structure

```
config/
├── client.toml    # Infrastructure: providers, servers, REST settings
├── model.toml     # Model settings: provider, model, prompt, tools
└── .env           # API keys (gitignored)
```

### Split Configuration Design

```mermaid
flowchart TB
    subgraph ConfigFiles["📁 Configuration Files"]
        direction LR
        CLIENT["📄 client.toml<br/>Infrastructure"]
        MODEL["📄 model.toml<br/>Model Settings"]
        ENV["🔐 .env<br/>Secrets"]
    end
    
    subgraph ClientFile["client.toml"]
        C1["[server]<br/>bind, cors_origins, docs"]
        C2["[[providers]]<br/>id, type, endpoint, models"]
        C3["[[servers]]<br/>name, command, args"]
    end
    
    subgraph ModelFile["model.toml"]
        M1["default_provider"]
        M2["model"]
        M3["prompt_template"]
        M4["[[tools]]<br/>name, description, server"]
    end
    
    CLIENT --> ClientFile
    MODEL --> ModelFile
    
    style ConfigFiles fill:#2d3436,stroke:#636e72,color:#fff
    style ClientFile fill:#0984e3,stroke:#74b9ff,color:#fff
    style ModelFile fill:#6c5ce7,stroke:#a29bfe,color:#fff
```

### client.toml - Infrastructure

```toml
# REST Server Settings
[server]
bind = "127.0.0.1:8080"          # Server bind address
cors_origins = [
    "http://localhost:5173",
]

[[server.docs]]
url = "http://localhost:8080"
description = "Local development"

# LLM Providers
[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "GEMINI_API_KEY"         # Environment variable name
models = [
    { name = "gemini-2.0-flash", display_name = "Gemini 2.0 Flash" }
]

# MCP Servers
[[servers]]
name = "time"
command = "/path/to/mcp-server-time"
default_timezone = "Asia/Jakarta"
```

### model.toml - Model Settings

```toml
# Default provider and model
default_provider = "gemini"
model = "gemini-2.0-flash"

# System prompt template
prompt_template = """
You are a helpful AI assistant.

{{custom_instruction}}

{{language_guidance}}

{{tool_guidance}}
"""

# Tools (synced from MCP servers)
[[tools]]
name = "get_current_time"
description = "Get current time"
server = "time"
```

### Configuration Flow

```mermaid
sequenceDiagram
    participant App as 🚀 Application
    participant Loader as 📂 Config Loader
    participant Client as 📄 client.toml
    participant Model as 📄 model.toml
    participant Env as 🔐 .env
    
    App->>Loader: load_config()
    Loader->>Env: Load environment variables
    Loader->>Client: Parse infrastructure config
    Loader->>Model: Parse model settings
    Loader->>Loader: Validate & merge configs
    Loader-->>App: AppConfig (complete)
```

### Full Configuration Reference

See example files:
- [config.example/client.toml](config.example/client.toml) - Infrastructure template
- [config.example/model.toml](config.example/model.toml) - Model settings template

---

## 🚀 Execution Modes

### Mode Overview

```mermaid
flowchart LR
    subgraph Binaries["📦 Available Binaries"]
        MENU["menu<br/>(default)"]
        STDIO["stdio"]
        REST["mcp-rest"]
    end
    
    subgraph Modes["🔄 Run Modes"]
        M1["💬 STDIO Only"]
        M2["🌐 REST Only"]
        M3["⚡ Both (STDIO + REST)"]
        M4["⚙️ Setup Menu"]
    end
    
    MENU -->|"Mode Selector"| M1
    MENU -->|"Mode Selector"| M2
    MENU -->|"Mode Selector"| M3
    MENU -->|"Mode Selector"| M4
    STDIO --> M1
    REST --> M2
    
    style Binaries fill:#2d3436,stroke:#636e72,color:#fff
    style Modes fill:#0984e3,stroke:#74b9ff,color:#fff
```

### Execution Flow

```mermaid
sequenceDiagram
    participant User as 👤 User
    participant CLI as 🖥️ CLI
    participant Config as 📂 Config
    participant App as 🚀 Application
    participant LLM as ☁️ LLM Provider
    participant MCP as 🔧 MCP Servers
    
    User->>CLI: cargo run [--mode rest]
    CLI->>Config: Load client.toml + model.toml
    Config-->>CLI: AppConfig
    
    alt STDIO Mode
        CLI->>App: Initialize TUI
        App->>MCP: Connect to servers
        MCP-->>App: Tools available
        loop Chat Loop
            User->>App: Send message
            App->>LLM: Request completion
            LLM-->>App: Response (may include tool calls)
            opt Tool Call
                App->>MCP: Execute tool
                MCP-->>App: Tool result
                App->>LLM: Send tool result
                LLM-->>App: Final response
            end
            App-->>User: Display response
        end
    else REST Mode
        CLI->>App: Start HTTP server
        App->>MCP: Connect to servers
        Note over App: Listening on config bind address
        loop Request Loop
            User->>App: HTTP POST /chat
            App->>LLM: Request completion
            LLM-->>App: Response
            App-->>User: JSON response
        end
    end
```

### Running Different Modes

```bash
# Default: TUI Mode Selector
cargo run

# Direct STDIO mode
cargo run --bin stdio

# Direct REST mode (uses config bind address)
cargo run --bin mcp-rest

# REST with CLI override
cargo run --bin mcp-rest -- --addr 0.0.0.0:3000

# Using mode flag
cargo run -- --mode rest
cargo run -- --mode stdio
cargo run -- --mode all
```

### Production Deployment

```mermaid
flowchart TB
    subgraph Production["🏭 Production Setup"]
        direction TB
        
        subgraph REST_OPT["REST API Optimization"]
            R1["Use mcp-rest binary"]
            R2["Configure bind in client.toml"]
            R3["Set CORS origins"]
            R4["Deploy behind reverse proxy"]
        end
        
        subgraph STDIO_OPT["STDIO/TUI Optimization"]
            S1["Use release build"]
            S2["Preload MCP servers"]
            S3["Configure default provider"]
        end
    end
    
    subgraph Config["📁 client.toml"]
        CFG["[server]<br/>bind = '0.0.0.0:8080'"]
    end
    
    REST_OPT --> CFG
    
    style Production fill:#1a1a2e,stroke:#6c5ce7,color:#fff
    style REST_OPT fill:#0984e3,stroke:#74b9ff,color:#fff
    style STDIO_OPT fill:#00b894,stroke:#00cec9,color:#fff
    style Config fill:#2d3436,stroke:#636e72,color:#fff
```

### REST API Optimization

| Aspect | Configuration | Description |
|:-------|:--------------|:------------|
| **Bind Address** | `[server].bind` | Configure in `client.toml` |
| **CORS** | `[server].cors_origins` | Whitelist allowed origins |
| **Binary** | `mcp-rest` | Optimized REST-only binary |
| **Logging** | `RUST_LOG=info` | Control log verbosity |

```bash
# Production REST deployment
RUST_LOG=info ./target/release/mcp-rest
```

### STDIO/TUI Optimization

| Aspect | Configuration | Description |
|:-------|:--------------|:------------|
| **Default Provider** | `model.toml` | Set preferred LLM |
| **Prompt Template** | `model.toml` | Customize system prompt |
| **Tools** | `model.toml` | Pre-configure available tools |
| **Binary** | `stdio` | Optimized STDIO-only binary |

```bash
# Production STDIO deployment
./target/release/stdio
```

---

## 🌐 API Reference

### REST Endpoints

| Method | Endpoint | Description |
|:------:|:---------|:------------|
| `POST` | `/chat` | 💬 Send chat message |
| `GET` | `/config` | ⚙️ Get configuration |
| `PUT` | `/config` | ✏️ Update configuration |
| `POST` | `/reload` | 🔄 Reload from file |
| `GET` | `/tools` | 🔧 List tools |
| `POST` | `/tools/{name}` | ▶️ Invoke tool |

> 📚 **Swagger UI**: `http://127.0.0.1:8080/swagger-ui/`

### STDIO/TUI Commands

| Command | Description |
|:--------|:------------|
| `/help` | 📖 Show commands |
| `/agent [on\|off]` | 🤖 Toggle agent mode |
| `/reset` | 🗑️ Clear session |
| `/logs` | 📋 Show interaction logs |
| `/steps` | 🔧 Show tool steps |
| `/exit` | 🚪 Exit app |

---

## 🔌 Adding MCP Servers

```mermaid
sequenceDiagram
    participant C as MCP Client
    participant S as MCP Server
    
    C->>S: Initialize
    S-->>C: Capabilities + Tools
    C->>S: tools/list
    S-->>C: Available Tools
    C->>S: tools/call
    S-->>C: Tool Result
```

### Steps

1️⃣ **Add server via Setup Menu or config**

```toml
[[servers]]
name = "my-server"
command = "/path/to/server-binary"
```

2️⃣ **Sync tools in Setup → Manage MCP Servers → Sync All**

3️⃣ **Tools auto-bind to the configured server**

---

## 🔍 Server Auto-Discovery

The discovery module provides automatic loading of MCP servers from a designated folder.

### Architecture

```mermaid
flowchart TB
    subgraph Discovery["🔍 Discovery Module"]
        direction TB
        SCAN["📂 Scanner<br/>scan_folder()"]
        LOAD["⚡ Loader<br/>load_all()"]
        TYPES["📦 Types<br/>DiscoveredServer"]
    end
    
    subgraph Folder["📁 servers/"]
        S1["mcp-time.exe"]
        S2["mcp-filesystem.exe"]
        S3["mcp-weather.exe"]
    end
    
    subgraph Process["🔄 Processing"]
        P1["1️⃣ Detect executable"]
        P2["2️⃣ Spawn process"]
        P3["3️⃣ MCP initialize"]
        P4["4️⃣ tools/list"]
    end
    
    subgraph Result["✅ Result"]
        R1["Server name"]
        R2["Tool list"]
        R3["Load status"]
    end
    
    Folder --> SCAN
    SCAN --> |"Find binaries"| LOAD
    LOAD --> Process
    Process --> TYPES
    TYPES --> Result
    
    style Discovery fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style Folder fill:#0984e3,stroke:#74b9ff,color:#fff
    style Process fill:#00b894,stroke:#00cec9,color:#fff
    style Result fill:#fd79a8,stroke:#e84393,color:#fff
```

### Discovery Flow

```mermaid
sequenceDiagram
    participant App as 🚀 Application
    participant Scanner as 📂 Scanner
    participant Loader as ⚡ Loader
    participant Server as 🔧 MCP Server
    
    App->>Scanner: scan_folder("servers")
    Scanner->>Scanner: Find executable files
    Scanner-->>App: Vec<DiscoveredServer>
    
    loop For each server
        App->>Loader: load_server(&mut server)
        Loader->>Server: Spawn process
        Loader->>Server: initialize
        Server-->>Loader: capabilities
        Loader->>Server: tools/list
        Server-->>Loader: available tools
        Loader-->>App: Update server.tools
    end
    
    App->>App: DiscoverySummary
```

### Usage

```rust
use antikhitera_mcp_client::application::discovery;

// Option 1: Scan and load in one step
let (servers, summary) = discovery::scan_and_load("servers").await?;

println!("✓ Loaded {} servers with {} total tools", 
    summary.loaded, 
    summary.total_tools
);

// Option 2: Manual two-step process
let mut servers = discovery::scan_folder("servers")?;
let summary = discovery::load_all(&mut servers).await;

// Iterate through results
for server in &servers {
    match &server.load_status {
        LoadStatus::Success => {
            println!("✓ {} - {} tools", server.name, server.tool_count());
            for (tool_name, desc) in &server.tools {
                println!("  • {}: {}", tool_name, desc);
            }
        }
        LoadStatus::Failed(e) => println!("✗ {} - {}", server.name, e),
        LoadStatus::NoTools => println!("⚠ {} - no tools", server.name),
        _ => {}
    }
}
```

### Module Structure

| File | Description |
|:-----|:------------|
| `discovery/mod.rs` | Module exports + documentation |
| `discovery/types.rs` | `DiscoveredServer`, `LoadStatus`, `DiscoverySummary` |
| `discovery/scanner.rs` | `scan_folder()` - Platform-specific executable detection |
| `discovery/loader.rs` | `load_all()`, `scan_and_load()` - MCP initialization |

### Platform-Specific Detection

| Platform | Detected Extensions |
|:---------|:-------------------|
| **Windows** | `.exe`, `.cmd`, `.bat` |
| **Unix/Linux** | Files with execute permission bit |

---

## 🧪 Development

### Project Structure

```mermaid
graph TD
    SRC[📂 src] ==> BIN[📦 bin]
    SRC ==> LIB[📚 lib]
    
    BIN --> MAIN("menu.rs<br/>Main Entry")
    BIN --> REST("rest.rs<br/>REST Only")
    BIN --> STD("stdio.rs<br/>STDIO Only")
    
    LIB --> APP[🎯 application]
    LIB --> CLI[🖥️ cli]
    LIB --> CFG[⚙️ config]
    LIB --> TUI[🖼️ tui]
    LIB --> INF[🔧 infrastructure]
    
    subgraph ConfigMod["Config Module"]
        LOADER["loader.rs<br/>Loads client.toml + model.toml"]
        WIZARD["wizard/"]
        GENS["generators/<br/>client.rs, model.rs"]
    end
    
    subgraph TUImod["TUI Modules (SOLID)"]
        SCREENS["screens/"]
        CHAT["chat/<br/>state, ui, input, runner"]
        SETUP_M["setup_menu/<br/>providers, models, servers, prompt"]
        WIDGETS["widgets/<br/>Menu, TableMenu"]
    end
    
    CFG --> ConfigMod
    TUI --> TUImod
    WIZARD --> GENS
    
    style SRC fill:#2d3436,stroke:#636e72,color:#fff
    style BIN fill:#0984e3,stroke:#74b9ff,color:#fff
    style LIB fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style TUImod fill:#00b894,stroke:#00cec9,color:#fff
    style ConfigMod fill:#fd79a8,stroke:#e84393,color:#fff
```

### Chat Module Architecture (SOLID)

```mermaid
classDiagram
    class ChatState {
        +messages: Vec~ChatMessage~
        +input: String
        +cursor_pos: usize
        +scroll_offset: u16
        +agent_mode: bool
        +loading: bool
        +add_message()
        +insert_char()
        +delete_char()
    }
    
    class ChatUI {
        +render(frame, state)
        -render_status_bar()
        -render_messages()
        -render_input()
        -render_help_bar()
    }
    
    class InputHandler {
        +handle_input(state, event): InputAction
        +parse_command(input): CommandResult
    }
    
    class ChatRunner {
        +run_chat(client, provider, model)
        -run_chat_loop()
        -send_message()
        -handle_command()
    }
    
    ChatRunner --> ChatState : manages
    ChatRunner --> ChatUI : renders
    ChatRunner --> InputHandler : processes
```

### 🛠️ Developer Commands

#### Build & Run

| Command | Description |
|:--------|:------------|
| `cargo build` | 🔨 Build in debug mode |
| `cargo build --release` | 🚀 Build for production |
| `cargo run` | 🎮 Run TUI mode selector |
| `cargo run --bin stdio` | 💬 Run directly in STDIO mode |
| `cargo run --bin rest` | 🌐 Run directly in REST mode |

#### Testing

| Command | Description |
|:--------|:------------|
| `cargo test` | 🧪 Run all tests |
| `cargo test --test config_loading_tests` | 📄 Run specific integration test |
| `cargo clippy` | 🔍 Lint code |
| `cargo fmt` | 🎨 Format code |

---

## 📄 License

MIT License - See [LICENSE](LICENSE) for details.

---

<p align="center">
  Made with ❤️ using Rust + Ratatui
</p>
