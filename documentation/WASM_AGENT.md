# WASM Agent

Arsitektur baru dimana WASM **hanya memproses LLM responses** dari host, tidak memanggil API langsung.

## Overview

### Sebelum (Old Architecture)
```
┌─────────────────────────────────┐
│  WASM                           │
│  ├─ Call LLM API ❌            │
│  ├─ Manage API Keys ❌         │
│  ├─ Handle Rate Limiting ❌    │
│  └─ Parse Response ✓           │
└─────────────────────────────────┘
```

### Sesudah (New Architecture)
```
┌──────────────────────────────────────────────┐
│  Host (TypeScript/Python/Go)                │
│  ┌────────────────────────────────────────┐ │
│  │  Host Imports (I/O):                   │ │
│  │  ├─ call-llm() → LLM API             │ │
│  │  ├─ emit-tool-call() → MCP Servers   │ │
│  │  ├─ log-message() → Console          │ │
│  │  └─ save/load-state() → Database     │ │
│  └────────────────────────────────────────┘ │
│           ↑              ↓                  │
│     WASM Imports    WASM Exports            │
└───────────┼──────────────┼──────────────────┘
            │              │
┌───────────▼──────────────┼──────────────────┐
│  WASM Component          │                  │
│  ┌─────────────────────┐ │                  │
│  │  Agent FSM Runner   │ │                  │
│  │  ├─ Parse LLM resp  │ │                  │
│  │  ├─ Extract actions │ │                  │
│  │  ├─ Validate JSON   │ │                  │
│  │  ├─ Build prompts   │ │                  │
│  │  └─ Manage state    │ │                  │
│  └─────────────────────┘ │                  │
│                          │                  │
│  Exports:                │                  │
│  ├─ agent-runner         │                  │
│  ├─ json-schema-validator│                  │
│  └─ prompt-manager       │                  │
└──────────────────────────┴──────────────────┘
```

## WIT Interface

### Host Imports (disediakan oleh host)

```wit
interface host-imports {
    /// Call LLM API (host responsibility)
    call-llm(request: llm-request) -> result<llm-response, string>;

    /// Execute tool call (host responsibility)
    emit-tool-call(event: tool-call-event) -> result<tool-execution-result, string>;

    /// Log message
    log-message(event: log-event);

    /// Save/load state
    save-state(session-id: string, state-json: string) -> result<_, string>;
    load-state(session-id: string) -> result<option<string>, string>;
}
```

### WASM Exports (disediakan oleh WASM)

```wit
interface agent-runner {
    /// Initialize agent
    init(config-json: string) -> result<string, string>;

    /// Process LLM response
    process-llm-response(
        session-id: string,
        llm-response-json: string,
    ) -> result<string, string>;

    /// Process tool result
    process-tool-result(
        session-id: string,
        tool-result-json: string,
    ) -> result<string, string>;

    /// Get agent state
    get-state(session-id: string) -> result<string, string>;

    /// Reset session
    reset-session(session-id: string) -> result<bool, string>;
}
```

## Config Structure

### WASM Config (Minimal)

```rust
WasmAgentConfig {
    agent: AgentConfig {
        max_steps: 10,
        verbose: false,
        auto_execute_tools: true,
        session_timeout_secs: 300,
        session_id: "session-1234567890",
    },
    prompts: PromptConfig {
        template: "You are a helpful AI...",
        tool_guidance: "...",
        // ... prompt fields
    },
    schemas: Vec<JsonSchemaConfig>,
    custom: HashMap<String, String>,
}
```

### CLI Config (for testing only)

```rust
CliConfig {
    providers: Vec<CliProviderConfig>,  // For testing
    default_provider: "openai",
    model: "gpt-4",
    server: ServerConfig { ... },
    custom: HashMap<String, String>,
}
```

## Flow Eksekusi

```
1. User Input
   ↓
2. Host: Build prompt, call LLM API
   ↓
3. Host: WASM.process_llm_response(llm_response_json)
   ↓
4. WASM: Parse JSON, extract action
   ├─ If "call_tool" → return CallTool action
   └─ If "final" → return FinalResponse action
   ↓
5. Host: If CallTool → execute tool via MCP
   ↓
6. Host: WASM.process_tool_result(tool_result_json)
   ↓
7. WASM: Build next prompt with tool result
   ↓
8. Host: Call LLM API again with new prompt
   ↓
9. Repeat until FinalResponse or max steps
```

## Usage Examples

### TypeScript Host

```typescript
import { instantiate } from '@bytecodealliance/jco';

// Load WASM
const wasmBytes = await readFile('antikythera.wasm');

// Instantiate with host imports
const instance = await instantiate(wasmBytes, {
  'antikythera-mcp-framework/host-imports': {
    'call-llm': async (request) => {
      // Call actual LLM API
      const response = await openai.chat.completions.create({
        model: 'gpt-4',
        messages: JSON.parse(request.messagesJson),
        temperature: request.temperature ?? 0.7,
      });

      return {
        content: response.choices[0].message.content,
        model: response.model,
        tokensUsed: response.usage?.total_tokens,
      };
    },

    'emit-tool-call': async (event) => {
      // Execute MCP tool
      const args = JSON.parse(event.argumentsJson);
      const result = await mcptools.execute(event.toolName, args);

      return {
        toolName: event.toolName,
        success: result.success,
        outputJson: JSON.stringify(result.output),
        errorMessage: result.error,
        stepId: event.stepId,
      };
    },

    'log-message': (event) => {
      console.log(`[${event.level}] ${event.message}`);
    },

    'save-state': async (sessionId, stateJson) => {
      await redis.set(`agent:${sessionId}`, stateJson);
    },

    'load-state': async (sessionId) => {
      return await redis.get(`agent:${sessionId}`);
    },
  }
});

// Initialize agent
await instance.exports.init(JSON.stringify({
  agent: {
    maxSteps: 10,
    verbose: true,
    autoExecuteTools: true,
    sessionTimeoutSecs: 300,
    sessionId: 'session-123',
  },
  prompts: { ... },
}));

// Process LLM response
const action = await instance.exports.processLlmResponse(
  'session-123',
  JSON.stringify({
    action: 'call_tool',
    tool: 'get_weather',
    input: { city: 'NYC' },
  })
);

console.log('Agent action:', JSON.parse(action));
```

### Rust SDK Usage

```rust
use antikythera_sdk::wasm_agent::*;

// Create agent state
let mut state = AgentState::new(AgentConfig {
    max_steps: 10,
    verbose: true,
    auto_execute_tools: true,
    session_timeout_secs: 300,
    session_id: "session-123".to_string(),
});

// Process LLM response
let llm_response = r#"{
    "action": "call_tool",
    "tool": "get_weather",
    "input": {"city": "NYC"}
}"#;

match process_llm_response(&mut state, llm_response) {
    Ok(AgentAction::CallTool { tool, input }) => {
        println!("Call tool: {} with {:?}", tool, input);
        // Host executes tool...
    }
    Ok(AgentAction::Final { response }) => {
        println!("Final response: {:?}", response);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Benefits

| Aspek | Old | New |
|-------|-----|-----|
| **Security** | WASM akses API keys | API keys tetap di host |
| **Portability** | WASM perlu HTTP stack | WASM kecil, fokus logic |
| **Flexibility** | Recompile WASM untuk provider baru | Host ganti provider tanpa recompile |
| **Rate Limiting** | WASM track limits | Host handle dengan Redis/memory |
| **Caching** | WASM implement cache | Host handle caching |
| **WASM Size** | Besar (include HTTP client) | Kecil (logic only) |

## Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `wasm-agent.pc` | WASM agent config | Project root |
| `cli-config.pc` | CLI testing config | Project root |
| `.env` | API keys (CLI only) | Project root |

## Migration

Untuk setup yang sudah ada:

1. **Pindahkan provider config ke CLI module**
2. **Hapus provider info dari WASM config**
3. **Host handle LLM API calls**
4. **WASM fokus pada agent logic**
