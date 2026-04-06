# WASM Configuration Binary Format

Efficient postcard-based binary configuration format for WASM deployments.

## Overview

All configuration stored in **single postcard binary blob** with clear internal structure:

```
┌─────────────────────────────────────────────┐
│  WasmConfig (postcard serialized)           │
│  ┌───────────────────────────────────────┐  │
│  │ ClientSection                         │  │
│  │ ├─ providers (Vec<ProviderConfig>)    │  │
│  │ ├─ servers (Vec<ServerConfig>)        │  │
│  │ ├─ rest_server (RestServerConfig)     │  │
│  │ └─ env_vars (HashMap)                 │  │
│  ├───────────────────────────────────────┤  │
│  │ ModelSection                          │  │
│  │ ├─ default_provider (String)          │  │
│  │ ├─ model (String)                     │  │
│  │ ├─ tools (Vec<ToolConfig>)            │  │
│  │ └─ model_params (HashMap)             │  │
│  ├───────────────────────────────────────┤  │
│  │ PromptSection                         │  │
│  │ ├─ template (String)                  │  │
│  │ ├─ tool_guidance (String)             │  │
│  │ ├─ fallback_guidance (String)         │  │
│  │ ├─ ... (10 prompt fields total)       │  │
│  ├───────────────────────────────────────┤  │
│  │ AgentSection                          │  │
│  │ ├─ max_steps (u32)                    │  │
│  │ ├─ timeout_secs (u32)                 │  │
│  │ ├─ verbose (bool)                     │  │
│  │ └─ metadata (HashMap)                 │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## Features

✅ **Single binary blob** - Easy to store and transfer  
✅ **Clear section structure** - Organized by purpose  
✅ **Efficient** - Postcard is compact and fast  
✅ **Type-safe** - Strongly typed Rust structs  
✅ **Easy to extend** - Add fields to sections  
✅ **Size tracking** - Built-in size breakdown utility  

## Usage

### Create Configuration

```rust
use antikythera_sdk::wasm_config::*;

let mut config = WasmConfig::default();

// Add providers
config.client.providers.push(ProviderConfig {
    id: "openai".to_string(),
    provider_type: "openai".to_string(),
    base_url: "https://api.openai.com/v1".to_string(),
    api_key: Some("sk-...".to_string()),
    headers: HashMap::new(),
});

// Set model
config.model.default_provider = "openai".to_string();
config.model.model = "gpt-4".to_string();

// Customize prompts
config.prompts.template = "You are a specialized assistant...".to_string();

// Configure agent
config.agent.max_steps = 20;
config.agent.verbose = true;
```

### Serialize to Binary

```rust
// Serialize to postcard binary
let binary: Vec<u8> = config_to_binary_simple(&config)
    .expect("Failed to serialize");

println!("Config size: {} bytes", binary.len());
```

### Deserialize from Binary

```rust
// Load from binary
let config: WasmConfig = config_from_binary_simple(&binary)
    .expect("Failed to deserialize");

// Use config
println!("Provider: {}", config.model.default_provider);
println!("Model: {}", config.model.model);
```

### Size Breakdown

```rust
let sizes = config_size_breakdown(&config);
println!("{:?}", sizes);
// Output: {"client": 256, "model": 128, "prompts": 512, "agent": 64}

// Summary
println!("{}", config_summary(&config));
```

Output:
```
WASM Configuration:
├─ Providers: 1
├─ Servers: 2
├─ Tools: 5
├─ Prompt sections: 10
├─ Max agent steps: 20
└─ Binary size breakdown:
   ├─ Client: 256 bytes
   ├─ Model: 128 bytes
   ├─ Prompts: 512 bytes
   ├─ Agent: 64 bytes
   └─ Total: 960 bytes
```

## Section Types

### ClientSection

Infrastructure configuration:
- **providers**: API provider definitions
- **servers**: MCP server definitions
- **rest_server**: REST API settings
- **env_vars**: Environment variables

### ModelSection

Model selection and tools:
- **default_provider**: Default provider ID
- **model**: Default model name
- **tools**: Available tool definitions
- **model_params**: Model-specific parameters

### PromptSection

All prompt templates (10 fields):
- **template**: Main system prompt
- **tool_guidance**: Tool usage guidance
- **fallback_guidance**: Out-of-scope handling
- **json_retry_message**: JSON parse error recovery
- **tool_result_instruction**: Result formatting
- **agent_instructions**: Agent behavior rules
- **ui_instructions**: UI hydration rules
- **language_instructions**: Language detection
- **agent_max_steps_error**: Max steps error
- **no_tools_guidance**: No tools message

### AgentSection

Agent behavior settings:
- **max_steps**: Maximum tool calls
- **timeout_secs**: Session timeout
- **verbose**: Enable logging
- **auto_execute_tools**: Auto-execute tools
- **session_id**: Session identifier
- **metadata**: Custom metadata

## Binary Format

For advanced usage with custom header and section table:

```rust
use antikythera_sdk::wasm_config::{
    config_to_binary, config_from_binary,
    CONFIG_MAGIC, CONFIG_VERSION
};

// Serialize with header
let binary = config_to_binary(&config)?;

// Header structure:
// Bytes 0-3:   Magic number (0xA7F9C3D1)
// Bytes 4-5:   Version (0x0001)
// Bytes 6-7:   Section count
// Bytes 8+:    Section data (postcard)

// Deserialize
let config = config_from_binary(&binary)?;
```

## WASM Integration

### JavaScript/TypeScript

```typescript
import { WasmClient } from 'antikythera-sdk';

// Load config from binary
const configBinary = await fetch('/config.bin').then(r => r.arrayBuffer());
const config = WasmConfig.fromBinary(new Uint8Array(configBinary));

// Create client with config
const client = new WasmClient(config);
```

### Rust WASM

```rust
use antikythera_sdk::wasm_config::*;

// Embed config in WASM binary
const CONFIG_BIN: &[u8] = include_bytes!("config.bin");

#[wasm_bindgen]
pub fn get_config() -> JsValue {
    let config = config_from_binary_simple(CONFIG_BIN).unwrap();
    serde_wasm_bindgen::to_value(&config).unwrap()
}
```

## Advantages Over TOML/JSON

| Feature | Postcard Binary | TOML/JSON |
|---------|----------------|-----------|
| **Size** | ~50% smaller | Larger |
| **Parse Speed** | Fast (zero-copy possible) | Slower |
| **Type Safety** | Compile-time checked | Runtime parsing |
| **Schema Evolution** | Additive changes | Manual migration |
| **Validation** | Type system | Custom validators |

## File Extension

Recommended: `.bin` or `.pc` (postcard)

Example: `config.bin`, `antikythera.pc`

## Tests

Run tests:

```bash
cargo test -p antikythera-sdk --features wasm-config wasm_config
```

Tests cover:
- Serialization/deserialization roundtrip
- Configuration with data
- Size breakdown accuracy
- Magic number validation
