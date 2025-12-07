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
    SETUP --> PROMPT["Edit Prompt Template"]
    
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
├── client.toml    # Main configuration
└── .env           # API keys (gitignored)
```

### Provider Configuration

```mermaid
graph LR
    subgraph Config["client.toml"]
        P1["[[providers]]<br/>id = 'gemini'<br/>type = 'gemini'"]
        P2["[[providers]]<br/>id = 'ollama'<br/>type = 'ollama'"]
        P3["[[providers]]<br/>id = 'openai'<br/>type = 'openai'"]
    end
    
    subgraph Backends["Backends"]
        B1["☁️ Google AI"]
        B2["🏠 Local Ollama"]
        B3["☁️ OpenAI API"]
    end
    
    P1 --> B1
    P2 --> B2
    P3 --> B3
    
    style Config fill:#2d3436,stroke:#636e72,color:#fff
    style Backends fill:#0984e3,stroke:#74b9ff,color:#fff
```

### Full Configuration Example

See [config.example/client.toml](config.example/client.toml) for the complete configuration reference.

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
    
    subgraph TUImod["TUI Modules (SOLID)"]
        SCREENS["screens/"]
        CHAT["chat/<br/>state, ui, input, runner"]
        SETUP_M["setup_menu/<br/>providers, models, servers, prompt"]
        WIDGETS["widgets/<br/>Menu, TableMenu"]
    end
    
    TUI --> TUImod
    
    style SRC fill:#2d3436,stroke:#636e72,color:#fff
    style BIN fill:#0984e3,stroke:#74b9ff,color:#fff
    style LIB fill:#6c5ce7,stroke:#a29bfe,color:#fff
    style TUImod fill:#00b894,stroke:#00cec9,color:#fff
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
