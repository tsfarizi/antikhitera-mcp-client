# antikythera-session

Session management and conversational data models for the Antikythera MCP Framework.

## Features

- `Message`, `MessageRole`, `MessagePart` — typed chat message model with custom serde (human-readable vs binary)
- `Session` — full session entity with messages, tokens, tools, steps
- `SessionManager` — thread-safe session manager (`Arc<RwLock<HashMap>>`) supporting concurrent operations
- `SessionExport` / `BatchExport` — versioned session import/export with Postcard binary format

## Session Lifecycle

```
new() → create_session() → add_message() → get_chat_history() → delete_session()
       → search_sessions() → export / import
```
