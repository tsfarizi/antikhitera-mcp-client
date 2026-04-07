# CLI Clean Architecture

Arsitektur CLI menggunakan **Clean Architecture** dengan dependency mengarah ke dalam.

## Struktur

```
antikythera-cli/src/
├── bin/
│   ├── menu.rs           # Main binary entry (antikythera)
│   └── config.rs         # Config CLI (antikythera-config)
│
├── lib.rs                # Module exports
│
├── domain/               # ⭐ INNERMOST - Core business logic
│   ├── mod.rs
│   ├── entities.rs       # Message, ProviderConfig, ChatSession, etc.
│   └── use_cases/
│       ├── mod.rs
│       └── chat_use_case.rs  # ChatUseCase, LlmProvider port, ToolExecutor port
│
├── infrastructure/       # ⭐ OUTER - External services
│   ├── mod.rs
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── gemini.rs     # GeminiProvider (implements LlmProvider port)
│   │   └── ollama.rs     # OllamaProvider (implements LlmProvider port)
│   └── config.rs         # Config loading utilities
│
├── presentation/         # ⭐ OUTER - UI layer
│   └── mod.rs            # TUI (to be reimplemented)
│
└── config/               # CLI-specific config
    └── mod.rs            # CliConfig (Gemini & Ollama only)
```

## Dependency Rule

```
Presentation → Infrastructure → Domain
                (implements)      (defines ports)
```

**Domain TIDAK tahu tentang Infrastructure atau Presentation.**

## Ports (Interfaces)

### LlmProvider Port

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn call(&self, messages: &[Message], system_prompt: &str)
        -> Result<String, Box<dyn Error + Send + Sync>>;
}
```

**Implementations:**
- `GeminiProvider` - Calls Google Gemini API
- `OllamaProvider` - Calls local Ollama API

### ToolExecutor Port

```rust
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, tool_call: &ToolCall)
        -> Result<ToolResult, Box<dyn Error + Send + Sync>>;
}
```

## Supported Providers

**Only Gemini and Ollama:**

| Provider | API Key Required | Default Endpoint |
|----------|-----------------|------------------|
| `gemini` | Yes | `https://generativelanguage.googleapis.com` |
| `ollama` | No | `http://127.0.0.1:11434` |

## Usage

### Initialize Config

```bash
# Create default config
antikythera-config init

# Add Gemini provider
antikythera-config add-provider gemini gemini https://generativelanguage.googleapis.com YOUR_API_KEY

# Add Ollama provider
antikythera-config add-provider ollama ollama http://127.0.0.1:11434

# Set default model
antikythera-config set-model gemini gemini-2.0-flash

# Show config
antikythera-config show
```

### Run CLI

```bash
# TUI mode (coming soon)
antikythera --mode tui

# REST mode (coming soon)
antikythera --mode rest
```

## Config Structure

```rust
CliConfig {
    providers: [
        CliProviderConfig {
            id: "gemini",
            provider_type: "gemini",
            endpoint: "https://generativelanguage.googleapis.com",
            api_key: "GEMINI_API_KEY",
            models: [ModelInfo { name: "gemini-2.0-flash", display_name: "Gemini 2.0 Flash" }],
        },
        CliProviderConfig {
            id: "ollama",
            provider_type: "ollama",
            endpoint: "http://127.0.0.1:11434",
            api_key: "",  // Not needed
            models: [ModelInfo { name: "llama3", display_name: "Llama 3" }],
        },
    ],
    default_provider: "gemini",
    model: "gemini-2.0-flash",
    server: ServerConfig {
        bind: "127.0.0.1:8080",
        cors_origins: [],
    },
}
```

## CLI as WASM Host

CLI binary bertindak sebagai **host** untuk WASM:

```
┌─────────────────────────────────────────┐
│  CLI Binary (Native)                   │
│  ┌───────────────────────────────────┐ │
│  │  Infrastructure Layer             │ │
│  │  ├─ GeminiProvider (HTTP client) │ │
│  │  ├─ OllamaProvider (HTTP client) │ │
│  │  └─ Config Loader (Postcard)     │ │
│  └──────────────┬────────────────────┘ │
│                 │                      │
│          LLM Request/Response          │
│                 │                      │
│  ┌──────────────▼────────────────────┐ │
│  │  Domain Layer                     │ │
│  │  ├─ ChatUseCase                  │ │
│  │  ├─ Agent FSM Logic              │ │
│  │  └─ JSON Parsing/Validation      │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## Benefits

| Aspek | Old (Tightly Coupled) | New (Clean Architecture) |
|-------|----------------------|-------------------------|
| **Testability** | Hard to mock LLM calls | Easy - inject mock LlmProvider |
| **Flexibility** | Provider hardcoded | Add new provider by implementing trait |
| **Maintainability** | Mixed concerns | Clear separation of concerns |
| **Providers** | Many providers | Only Gemini & Ollama (minimal) |
| **Config** | Complex TOML split | Simple Postcard binary |
