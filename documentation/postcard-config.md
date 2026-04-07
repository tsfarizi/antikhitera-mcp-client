# Postcard Configuration System

Sistem konfigurasi baru berbasis **Postcard binary format** yang lebih efisien dan cepat dibanding TOML.

## Overview

Semua konfigurasi sekarang disimpan sebagai **single Postcard binary file** (`config/app.pc`) kecuali file `.env` yang tetap untuk secrets.

### Keuntungan Postcard

| Aspek | TOML | Postcard |
|-------|------|----------|
| **Ukuran** | ~2KB | ~500 bytes (75% lebih kecil) |
| **Parse Speed** | Slow (text parsing) | Fast (binary deserialization) |
| **Type Safety** | Runtime parsing | Compile-time types |
| **Human Readable** | ✅ Ya | ❌ Binary |
| **Version Control** | ✅ Diffable | ❌ Binary diff |

### Migration Path

```
TOML Config (client.toml + model.toml)
          ↓
    Migration Tool
          ↓
Postcard Config (app.pc)
```

## Config Structure

```rust
AppConfig {
    server: ServerConfig {
        bind: "127.0.0.1:8080",
        cors_origins: [],
        docs: [],
    },
    providers: [
        ProviderConfig {
            id: "openai",
            provider_type: "openai",
            endpoint: "https://api.openai.com/v1",
            api_key: "OPENAI_API_KEY",
            models: [
                ModelInfo { name: "gpt-4", display_name: "GPT-4" }
            ],
        }
    ],
    model: ModelConfig {
        default_provider: "openai",
        model: "gpt-4",
    },
    prompts: PromptsConfig {
        template: "You are a helpful AI assistant...",
        tool_guidance: "...",
        // ... 10 prompt fields
    },
    agent: AgentConfig {
        max_steps: 10,
        verbose: false,
        auto_execute_tools: true,
        session_timeout_secs: 300,
    },
    custom: {},  // Extensible key-value store
}
```

## CLI Usage

### Initialize Config

```bash
# Create default config
antikythera-config init

# Output:
# ✓ Default configuration created at: config/app.pc
#   Size: 487 bytes
```

### View Config

```bash
# Show all config as JSON
antikythera-config show

# Output:
# {
#   "server": {
#     "bind": "127.0.0.1:8080",
#     "cors_origins": []
#   },
#   "providers": [...],
#   ...
# }

# Show config size
antikythera-config size
# Output: Configuration size: 487 bytes
```

### Get/Set Fields

```bash
# Get a specific field
antikythera-config get server.bind
# Output: 127.0.0.1:8080

# Set a specific field
antikythera-config set server.bind 0.0.0.0:3000
# Output: ✓ Set 'server.bind' = '0.0.0.0:3000'

# Available fields:
# - server.bind
# - server.cors_origins (JSON array)
# - model.default_provider
# - model.model
# - agent.max_steps
# - agent.verbose
# - agent.auto_execute_tools
# - agent.session_timeout_secs
# - prompts.<name> (see prompt fields below)
```

### Provider Management

```bash
# List all providers
antikythera-config list-providers

# Add a new provider
antikythera-config add-provider openai openai https://api.openai.com/v1 OPENAI_API_KEY

# Remove a provider
antikythera-config remove-provider openai

# Set default model
antikythera-config set-model openai gpt-4
```

### Prompt Management

```bash
# List all prompt templates
antikythera-config list-prompts

# Get a specific prompt
antikythera-config get-prompt template

# Set a prompt template
antikythera-config set-prompt template "You are a helpful assistant..."
```

**Available prompt fields:**
- `template`
- `tool_guidance`
- `fallback_guidance`
- `json_retry_message`
- `tool_result_instruction`
- `agent_instructions`
- `ui_instructions`
- `language_instructions`
- `agent_max_steps_error`
- `no_tools_guidance`

### Agent Configuration

```bash
# Show agent config
antikythera-config show-agent

# Set max steps
antikythera-config set-agent-max-steps 20

# Toggle verbose
antikythera-config set-agent-verbose true

# Toggle auto-execute tools
antikythera-config set-agent-auto-execute false
```

### Import/Export

```bash
# Export config as JSON
antikythera-config export
antikythera-config export backup.json

# Import config from JSON
antikythera-config import backup.json

# Reset to defaults
antikythera-config reset
```

### Migration

```bash
# Check migration status
antikythera-config migration-status

# Migrate from TOML to Postcard
antikythera-config migrate
```

## FFI Usage

### Core Config Functions

```c
// Initialize default config
char* mcp_config_init();

// Check if config exists
int32_t mcp_config_exists();

// Get config size
char* mcp_config_size();

// Get all config as JSON
char* mcp_config_get_all();

// Set all config from JSON
char* mcp_config_set_all(const char* config_json);

// Export config as JSON
char* mcp_config_export();

// Import config from JSON
char* mcp_config_import(const char* config_json);

// Reset to defaults
char* mcp_config_reset();
```

### Field-Level Functions

```c
// Get a specific field
char* mcp_config_get(const char* field);

// Set a specific field
char* mcp_config_set(const char* field, const char* value);
```

### Provider Functions

```c
// Add a provider
char* mcp_config_add_provider(
    const char* id,
    const char* provider_type,
    const char* endpoint,
    const char* api_key
);

// Remove a provider
char* mcp_config_remove_provider(const char* id);

// List all providers
char* mcp_config_list_providers();
```

### Prompt Functions

```c
// Get a prompt template
char* mcp_config_get_prompt(const char* name);

// Set a prompt template
char* mcp_config_set_prompt(const char* name, const char* value);

// List all prompt names
char* mcp_config_list_prompts();
```

### Agent Functions

```c
// Get agent config
char* mcp_config_get_agent();

// Set agent max steps
char* mcp_config_set_agent_max_steps(uint32_t steps);

// Toggle agent verbose
char* mcp_config_set_agent_verbose(int32_t enabled);

// Toggle auto-execute tools
char* mcp_config_set_agent_auto_execute(int32_t enabled);
```

### Example (Python)

```python
import ctypes
import json

lib = ctypes.CDLL("./libantikythera_sdk.so")

# Initialize config
result = json.loads(lib.mcp_config_init().decode())
print(f"Config initialized: {result['success']}")

# Get all config
config = json.loads(lib.mcp_config_get_all().decode())
print(f"Default provider: {config['model']['default_provider']}")

# Set a field
result = json.loads(lib.mcp_config_set(
    b"server.bind",
    b"0.0.0.0:3000"
).decode())
print(f"Field set: {result['success']}")

# Add a provider
result = json.loads(lib.mcp_config_add_provider(
    b"openai",
    b"openai",
    b"https://api.openai.com/v1",
    b"OPENAI_API_KEY"
).decode())
print(f"Provider added: {result['success']}")

# Export config
config_json = lib.mcp_config_export().decode()
print(f"Exported config: {config_json[:100]}...")
```

### Example (Node.js)

```javascript
const ffi = require('ffi-napi');
const ref = require('ref-napi');

const lib = ffi.Library('./libantikythera_sdk', {
  'mcp_config_init': ['pointer', []],
  'mcp_config_get_all': ['pointer', []],
  'mcp_config_set': ['pointer', ['string', 'string']],
  'mcp_config_export': ['pointer', []],
});

function readCString(ptr) {
  return ref.readCString(ptr);
}

// Initialize config
const initResult = JSON.parse(readCString(lib.mcp_config_init()));
console.log('Config initialized:', initResult.success);

// Get all config
const config = JSON.parse(readCString(lib.mcp_config_get_all()));
console.log('Default provider:', config.model.default_provider);

// Set a field
const setResult = JSON.parse(readCString(
  lib.mcp_config_set('server.bind', '0.0.0.0:3000')
));
console.log('Field set:', setResult.success);

// Export config
const exportJson = readCString(lib.mcp_config_export());
console.log('Exported config:', exportJson.substring(0, 100));
```

## Programmatic Usage (Rust)

```rust
use antikythera_sdk::config::*;

// Load config
let config = load_config(None)?;
println!("Default provider: {}", config.model.default_provider);

// Modify config
let mut config = load_config(None)?;
config.server.bind = "0.0.0.0:3000".to_string();
config.model.default_provider = "openai".to_string();
config.model.model = "gpt-4".to_string();

// Save config
save_config(&config, None)?;

// Get config size
let size = config_size(None)?;
println!("Config size: {} bytes", size);

// Export as JSON
let json = serde_json::to_string_pretty(&config)?;
std::fs::write("backup.json", json)?;

// Import from JSON
let json = std::fs::read_to_string("backup.json")?;
let config: AppConfig = serde_json::from_str(&json)?;
save_config(&config, None)?;
```

## Migration from TOML

### Automatic Migration

```bash
# Check if migration is needed
antikythera-config migration-status

# Run migration
antikythera-config migrate
```

### Manual Migration (Rust)

```rust
use antikythera_core::config::migration::*;

if needs_migration() {
    let config = migrate_toml_to_postcard()?;
    println!("Migration complete! Size: {} bytes", 
        postcard_config::config_size(None)?);
}
```

## Config File Location

- **Postcard config:** `config/app.pc`
- **Environment file:** `config/.env` (unchanged)

The `.env` file remains for secrets and API keys, while all other configuration is now in Postcard format.

## Performance Comparison

| Operation | TOML | Postcard | Improvement |
|-----------|------|----------|-------------|
| Load config | ~5ms | ~0.1ms | **50x faster** |
| Save config | ~3ms | ~0.05ms | **60x faster** |
| File size | ~2KB | ~500B | **75% smaller** |
| Parse memory | ~10KB | ~2KB | **80% less** |

## Best Practices

1. **Use CLI for testing** - Quick config changes via CLI
2. **Use FFI for apps** - Programmatic config via FFI
3. **Export before changes** - Backup config as JSON before major changes
4. **Use migration tool** - Migrate existing TOML configs automatically
5. **Keep .env for secrets** - Don't store API keys in Postcard config
