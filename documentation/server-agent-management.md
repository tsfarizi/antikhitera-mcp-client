# Server & Agent Management via WASM FFI

Dokumentasi untuk mengelola MCP Server dan Multi-Agent melalui WASM FFI interface.

## Overview

Sistem ini memungkinkan host language untuk:
- Menambah/menghapus MCP Server secara dinamis
- Mendaftarkan/menghapus Agent dengan berbagai role
- Konfigurasi output format response (JSON/Markdown/Text)
- Export/Import konfigurasi sebagai JSON

Semua konfigurasi divalidasi ketat sebelum digunakan.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Host Language (TypeScript/Python/etc)      │
│  ┌───────────────────────────────────────┐ │
│  │  MCP Server Management                │ │
│  │  ├─ Add/Remove servers                │ │
│  │  ├─ Start/Stop servers                │ │
│  │  ├─ Validate configs                  │ │
│  │  └─ Export/Import configs             │ │
│  ├───────────────────────────────────────┤ │
│  │  Agent Management                     │ │
│  │  ├─ Register/Unregister agents        │ │
│  │  ├─ Execute tasks (single/parallel)   │ │
│  │  ├─ Multi-agent orchestration         │ │
│  │  └─ Agent status tracking             │ │
│  └───────────────────────────────────────┘ │
│           ↑              ↓                 │
│     WASM Exports    WASM Exports           │
└───────────┼──────────────┼─────────────────┘
            │              │
┌───────────▼──────────────┼─────────────────┐
│  WASM Component          │                 │
│  ┌─────────────────────┐ │                 │
│  │  Server Manager     │ │                 │
│  │  ├─ Registry        │ │                 │
│  │  ├─ Validation      │ │                 │
│  │  └─ FFI Interface   │ │                 │
│  ├─────────────────────┤ │                 │
│  │  Agent Manager      │ │                 │
│  │  ├─ Registry        │ │                 │
│  │  ├─ Orchestration   │ │                 │
│  │  └─ FFI Interface   │ │                 │
│  └─────────────────────┘ │                 │
└──────────────────────────┴─────────────────┘
```

## Server Management

### Types

#### Server Transport
```wit
variant server-transport {
    stdio,
    http,
    sse,
}
```

#### Server Configuration
```wit
record mcp-server-config {
    name: string,
    transport: server-transport,
    command: string,
    args: list<string>,
    env: list<tuple<string, string>>,
    timeout-ms: option<u32>,
    enabled: bool,
    description: option<string>,
}
```

#### Validation Result
```wit
record server-validation-result {
    valid: bool,
    errors: list<string>,
    server-name: string,
}
```

### FFI Functions

#### `mcp_add_server`

Add a new MCP server with validation.

```c
char* mcp_add_server(const char* config_json);
```

**Returns:**
- JSON `server-validation-result`

**Example (Python):**
```python
import ctypes
import json

lib = ctypes.CDLL("./libantikythera_sdk.so")

config = {
    "name": "my-tool-server",
    "transport": "Stdio",
    "command": "node",
    "args": ["server.js"],
    "env": [["PORT", "3000"]],
    "timeout_ms": 5000,
    "enabled": True,
    "description": "My custom MCP server"
}

result_ptr = lib.mcp_add_server(json.dumps(config).encode())
result = json.loads(result_ptr.decode())

if result["valid"]:
    print(f"Server '{result['server_name']}' added successfully")
else:
    print(f"Validation failed: {result['errors']}")
```

#### `mcp_remove_server`

Remove an MCP server by name.

```c
char* mcp_remove_server(const char* name);
```

#### `mcp_list_servers`

List all configured MCP servers.

```c
char* mcp_list_servers();
```

**Returns:**
- JSON array of `mcp-server-config`

#### `mcp_get_server`

Get configuration for a specific server.

```c
char* mcp_get_server(const char* name);
```

#### `mcp_validate_server`

Validate server configuration without adding.

```c
char* mcp_validate_server(const char* config_json);
```

#### `mcp_export_servers_config`

Export all servers as JSON.

```c
char* mcp_export_servers_config();
```

#### `mcp_import_servers_config`

Import servers from JSON.

```c
char* mcp_import_servers_config(const char* config_json);
```

### Validation Rules

| Field | Rule |
|-------|------|
| `name` | Alphanumeric + `-` + `_` only, not empty |
| `command` | Not empty |
| `transport` | Must be `stdio`, `http`, or `sse` |
| HTTP/SSE `command` | Must start with `http://` or `https://` |
| `timeout_ms` | Must be > 0 if present |

## Agent Management

### Types

#### Agent Type
```wit
variant agent-type {
    general-assistant,
    code-reviewer,
    data-analyst,
    researcher,
    custom,
}
```

#### Skill Level
```wit
variant skill-level {
    beginner,
    intermediate,
    expert,
}
```

#### Agent Capability
```wit
record agent-capability {
    name: string,
    level: skill-level,
    description: string,
}
```

#### Agent Configuration
```wit
record agent-config {
    id: string,
    agent-type: agent-type,
    name: string,
    description: option<string>,
    model-provider: string,
    model: string,
    max-steps: u32,
    can-call-tools: bool,
    capabilities: list<agent-capability>,
    custom-prompt: option<string>,
    temperature: option<f32>,
    enabled: bool,
}
```

### FFI Functions

#### `mcp_register_agent`

Register a new agent with validation.

```c
char* mcp_register_agent(const char* config_json);
```

**Returns:**
- JSON `agent-validation-result`

**Example (Python):**
```python
config = {
    "id": "code-reviewer-v1",
    "agent-type": "CodeReviewer",
    "name": "Code Reviewer",
    "description": "Specialized code review agent",
    "model-provider": "openai",
    "model": "gpt-4",
    "max-steps": 15,
    "can-call-tools": True,
    "capabilities": [
        {
            "name": "code-review",
            "level": "Expert",
            "description": "Expert code review capabilities"
        }
    ],
    "custom-prompt": "You are a code review expert...",
    "temperature": 0.3,
    "enabled": True
}

result_ptr = lib.mcp_register_agent(json.dumps(config).encode())
result = json.loads(result_ptr.decode())

if result["valid"]:
    print(f"Agent '{result['agent_id']}' registered successfully")
else:
    print(f"Validation failed: {result['errors']}")
```

#### `mcp_unregister_agent`

Unregister an agent by ID.

```c
char* mcp_unregister_agent(const char* id);
```

#### `mcp_list_agents`

List all registered agents.

```c
char* mcp_list_agents();
```

**Returns:**
- JSON array of `agent-config`

#### `mcp_get_agent`

Get configuration for a specific agent.

```c
char* mcp_get_agent(const char* id);
```

#### `mcp_get_agent_status`

Get runtime status of all agents.

```c
char* mcp_get_agent_status();
```

**Returns:**
- JSON array of `agent-status`

#### `mcp_validate_agent`

Validate agent configuration without registering.

```c
char* mcp_validate_agent(const char* config_json);
```

#### `mcp_export_agents_config`

Export all agents as JSON.

```c
char* mcp_export_agents_config();
```

#### `mcp_import_agents_config`

Import agents from JSON.

```c
char* mcp_import_agents_config(const char* config_json);
```

### Validation Rules

| Field | Rule |
|-------|------|
| `id` | Alphanumeric + `-` + `_` only, not empty |
| `name` | Not empty |
| `model-provider` | Not empty |
| `model` | Not empty |
| `max-steps` | Must be > 0 |
| `temperature` | Must be 0.0 - 2.0 if present |

## Response Formatting

### Types

#### Output Format
```wit
variant output-format {
    json,
    markdown,
    text,
}
```

### FFI Functions

#### `mcp_set_output_format`

Set the output format for a server's responses.

```c
int32_t mcp_set_output_format(uint32_t server_id, const char* format);
```

**Parameters:**
- `format` - One of: `"json"`, `"markdown"`, `"text"`

#### `mcp_get_output_format`

Get current output format for a server.

```c
char* mcp_get_output_format(uint32_t server_id);
```

#### `mcp_format_response`

Format a response according to the server's output format.

```c
char* mcp_format_response(uint32_t server_id, const char* content, const char* data_json);
```

## Output Format Examples

### JSON Format
```json
{
  "content": "The weather is 72°F",
  "data": {"temp": 72, "unit": "F"},
  "format": "json"
}
```

### Markdown Format
```markdown
# Response

The weather is 72°F

## Data

```json
{"temp": 72, "unit": "F"}
```
```

### Text Format
```
The weather is 72°F

Data:
{"temp": 72, "unit": "F"}
```

## Complete Workflow Example

```python
import ctypes
import json

lib = ctypes.CDLL("./libantikythera_sdk.so")

# 1. Add a server
server_config = {
    "name": "weather-server",
    "transport": "Http",
    "command": "http://localhost:3000",
    "args": [],
    "env": [],
    "timeout_ms": 5000,
    "enabled": True,
    "description": "Weather API server"
}
result = json.loads(lib.mcp_add_server(json.dumps(server_config).encode()).decode())
print(f"Server added: {result['valid']}")

# 2. Register an agent
agent_config = {
    "id": "weather-agent",
    "agent-type": "DataAnalyst",
    "name": "Weather Agent",
    "description": "Fetches weather data",
    "model-provider": "openai",
    "model": "gpt-4",
    "max-steps": 10,
    "can-call-tools": True,
    "capabilities": [
        {"name": "weather-fetch", "level": "Expert", "description": "Fetch weather data"}
    ],
    "custom-prompt": None,
    "temperature": 0.5,
    "enabled": True
}
result = json.loads(lib.mcp_register_agent(json.dumps(agent_config).encode()).decode())
print(f"Agent registered: {result['valid']}")

# 3. Set output format to JSON
server_id = 1
lib.mcp_set_output_format(server_id, b"json")

# 4. List all servers
servers = json.loads(lib.mcp_list_servers().decode())
print(f"Total servers: {len(servers)}")

# 5. List all agents
agents = json.loads(lib.mcp_list_agents().decode())
print(f"Total agents: {len(agents)}")

# 6. Export configuration
servers_json = lib.mcp_export_servers_config().decode()
agents_json = lib.mcp_export_agents_config().decode()

# 7. Cleanup
lib.mcp_remove_server(b"weather-server")
lib.mcp_unregister_agent(b"weather-agent")
```

## Error Handling

All functions return JSON on error:
```json
{"error": "Server 'xyz' not found"}
{"error": "Invalid JSON: ..."}
```

Always parse the response and check for `error` field!

## Thread Safety

- ✅ Server registry: Mutex-protected
- ✅ Agent registry: Mutex-protected
- ✅ Output formats: Mutex-protected
- ✅ All FFI functions: Thread-safe
