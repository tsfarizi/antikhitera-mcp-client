# Configuration

This folder contains example configuration files for the MCP client.

## Setup

1. Copy the example files to `config/`:

```bash
mkdir -p config
cp config.example/client.toml config/client.toml
cp config.example/.env config/.env
```

2. Edit `config/.env` and set your API keys:

```env
GEMINI_API_KEY=your_actual_api_key
```

3. Edit `config/client.toml` to configure:
   - Default provider and model
   - Prompt template
   - MCP servers
   - Tool bindings

## Configuration Files

| File | Description |
|------|-------------|
| `client.toml` | Main configuration (providers, servers, tools) |
| `.env` | Environment variables (API keys) |

## Adding MCP Servers

Add a server in `client.toml`:

```toml
[[servers]]
name = "my-server"
command = "/path/to/mcp-server-binary"
args = ["--optional-flag"]
```

Then bind tools to the server:

```toml
[[tools]]
name = "my_tool"
description = "What this tool does"
server = "my-server"
```
