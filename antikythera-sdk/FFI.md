# FFI (Foreign Function Interface) Documentation

Expose REST server functionality to any language with C ABI support.

## Overview

The FFI layer provides `extern "C"` functions that can be called from:
- **Python** (ctypes, cffi)
- **Node.js** (ffi-napi)
- **Go** (cgo)
- **Java** (JNA)
- **C#** (P/Invoke)
- **Ruby** (FFI gem)
- And more...

## Building

### As Shared Library

```bash
# Linux
cargo build --release --features ffi --lib
# Output: target/release/libantikythera_sdk.so

# Windows
cargo build --release --features ffi --lib
# Output: target/release/antikythera_sdk.dll

# macOS
cargo build --release --features ffi --lib
# Output: target/release/libantikythera_sdk.dylib
```

## API Reference

### Server Lifecycle

#### `mcp_server_create`

Create a new MCP REST server.

**Signature:**
```c
uint32_t mcp_server_create(const char* addr);
```

**Parameters:**
- `addr` - Bind address (e.g., "127.0.0.1:8080")

**Returns:**
- Server ID (u32), or 0 on error

**Example (Python):**
```python
import ctypes

lib = ctypes.CDLL("./libantikythera_sdk.so")
server_id = lib.mcp_server_create(b"127.0.0.1:8080")
assert server_id != 0
```

---

#### `mcp_server_create_with_cors`

Create server with CORS configuration.

**Signature:**
```c
uint32_t mcp_server_create_with_cors(const char* addr, const char* cors_origins);
```

**Parameters:**
- `addr` - Bind address
- `cors_origins` - Comma-separated origins (e.g., "http://localhost:3000,https://example.com")

**Example (Node.js):**
```javascript
const ffi = require('ffi-napi');

const lib = ffi.Library('./libantikythera_sdk', {
  'mcp_server_create_with_cors': ['uint32', ['string', 'string']]
});

const serverId = lib.mcp_server_create_with_cors(
  '0.0.0.0:3000',
  'http://localhost:3000,https://app.example.com'
);
```

---

#### `mcp_server_is_running`

Check if server is running.

**Signature:**
```c
int32_t mcp_server_is_running(uint32_t server_id);
```

**Returns:**
- 1 if running, 0 if not

---

#### `mcp_server_stop`

Stop a server.

**Signature:**
```c
int32_t mcp_server_stop(uint32_t server_id);
```

**Returns:**
- 1 on success, 0 on error

---

#### `mcp_server_stop_all`

Stop all servers.

**Signature:**
```c
uint32_t mcp_server_stop_all(void);
```

**Returns:**
- Number of servers stopped

---

### Chat Operations

#### `mcp_server_chat`

Send a chat request.

**Signature:**
```c
char* mcp_server_chat(uint32_t server_id, const char* request_json, size_t request_len);
```

**Parameters:**
- `server_id` - Server ID
- `request_json` - JSON request body
- `request_len` - Length of JSON

**Returns:**
- Pointer to response JSON (must free with `mcp_string_free()`)

**Request Format:**
```json
{
  "prompt": "Your message",
  "agent": false,
  "session_id": "optional-session-id",
  "max_tool_steps": 10,
  "system_prompt": "Optional system prompt",
  "debug": false,
  "attachments": [
    {
      "name": "image.png",
      "mime_type": "image/png",
      "data": "base64-encoded-data"
    }
  ]
}
```

**Response Format:**
```json
{
  "status": "ok",
  "prompt": "Your message",
  "agent": false,
  "session_id": "optional-session-id",
  "content": "AI response here",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

**Example (Go):**
```go
package main

/*
#cgo LDFLAGS: -L../target/release -lantikythera_sdk
#include <stdint.h>
#include <stdlib.h>

extern uint32_t mcp_server_create(const char*);
extern char* mcp_server_chat(uint32_t, const char*, size_t);
extern void mcp_string_free(char*);
*/
import "C"
import "fmt"

func main() {
    addr := C.CString("127.0.0.1:8080")
    defer C.free(unsafe.Pointer(addr))
    
    serverId := C.mcp_server_create(addr)
    
    request := C.CString(`{"prompt": "Hello", "agent": false}`)
    defer C.free(unsafe.Pointer(request))
    
    response := C.mcp_server_chat(serverId, request, C.size_t(len(`{"prompt": "Hello", "agent": false}`)))
    defer C.mcp_string_free(response)
    
    fmt.Println(C.GoString(response))
}
```

---

### Tools

#### `mcp_server_get_tools`

Get available tools.

**Signature:**
```c
char* mcp_server_get_tools(uint32_t server_id);
```

**Returns:**
- JSON string with tools list (must free)

---

### Configuration

#### `mcp_server_get_config`

Get server configuration.

**Signature:**
```c
char* mcp_server_get_config(uint32_t server_id);
```

---

#### `mcp_server_reload`

Reload configuration.

**Signature:**
```c
char* mcp_server_reload(uint32_t server_id);
```

---

#### `mcp_server_update_config`

Update configuration.

**Signature:**
```c
char* mcp_server_update_config(uint32_t server_id, const char* config_json);
```

---

### Error Handling

#### `mcp_last_error`

Get last error message.

**Signature:**
```c
const char* mcp_last_error(void);
```

**Returns:**
- Error string (do NOT free)

---

#### `mcp_clear_error`

Clear last error.

**Signature:**
```c
void mcp_clear_error(void);
```

---

### Memory Management

#### `mcp_string_free`

Free a string returned by the library.

**Signature:**
```c
void mcp_string_free(char* ptr);
```

**IMPORTANT:** Always call this on strings returned by FFI functions!

---

### Utility

#### `mcp_version`

Get library version.

**Signature:**
```c
const char* mcp_version(void);
```

---

#### `mcp_server_count`

Get active server count.

**Signature:**
```c
uint32_t mcp_server_count(void);
```

---

#### `mcp_server_list`

List all server IDs.

**Signature:**
```c
uint32_t mcp_server_list(uint32_t* buffer, size_t buffer_len);
```

**Returns:**
- Number of servers written to buffer

---

## Complete Examples

### Python (ctypes)

```python
import ctypes
import json

# Load library
lib = ctypes.CDLL("./libantikythera_sdk.so")

# Define function signatures
lib.mcp_server_create.argtypes = [ctypes.c_char_p]
lib.mcp_server_create.restype = ctypes.c_uint32

lib.mcp_server_chat.argtypes = [ctypes.c_uint32, ctypes.c_char_p, ctypes.c_size_t]
lib.mcp_server_chat.restype = ctypes.c_char_p

lib.mcp_string_free.argtypes = [ctypes.c_char_p]

lib.mcp_last_error.argtypes = []
lib.mcp_last_error.restype = ctypes.c_char_p

# Create server
server_id = lib.mcp_server_create(b"127.0.0.1:8080")
if server_id == 0:
    error = lib.mcp_last_error()
    raise RuntimeError(f"Failed to create server: {error}")

try:
    # Send chat request
    request = json.dumps({
        "prompt": "What is the capital of France?",
        "agent": False
    }).encode()
    
    response_ptr = lib.mcp_server_chat(server_id, request, len(request))
    response = json.loads(response_ptr.decode())
    lib.mcp_string_free(response_ptr)
    
    print(f"Response: {response['content']}")

finally:
    # Cleanup
    lib.mcp_server_stop(server_id)
```

### Node.js (ffi-napi)

```javascript
const ffi = require('ffi-napi');
const ref = require('ref-napi');

const lib = ffi.Library('./libantikythera_sdk', {
  'mcp_server_create': ['uint32', ['string']],
  'mcp_server_chat': ['pointer', ['uint32', 'pointer', 'uint32']],
  'mcp_server_stop': ['int32', ['uint32']],
  'mcp_string_free': ['void', ['pointer']],
  'mcp_last_error': ['pointer', []],
});

function readCString(ptr) {
  if (ptr.isNull()) return null;
  return ref.readCString(ptr);
}

// Create server
const serverId = lib.mcp_server_create('127.0.0.1:8080');
if (serverId === 0) {
  const error = readCString(lib.mcp_last_error());
  throw new Error(`Failed: ${error}`);
}

// Chat
const request = JSON.stringify({ prompt: 'Hello', agent: false });
const requestBuffer = Buffer.from(request);

const responsePtr = lib.mcp_server_chat(serverId, requestBuffer, requestBuffer.length);
const response = JSON.parse(readCString(responsePtr));

// Free memory
lib.mcp_string_free(responsePtr);

console.log('Response:', response.content);

// Cleanup
lib.mcp_server_stop(serverId);
```

### C# (P/Invoke)

```csharp
using System;
using System.Runtime.InteropServices;
using System.Text.Json;

class Program
{
    [DllImport("antikythera_sdk")]
    static extern uint mcp_server_create(string addr);
    
    [DllImport("antikythera_sdk")]
    static extern IntPtr mcp_server_chat(uint serverId, string requestJson, nuint requestLen);
    
    [DllImport("antikythera_sdk")]
    static extern void mcp_string_free(IntPtr ptr);
    
    [DllImport("antikythera_sdk")]
    static extern int mcp_server_stop(uint serverId);
    
    [DllImport("antikythera_sdk")]
    static extern IntPtr mcp_last_error();
    
    [DllImport("antikythera_sdk")]
    static extern IntPtr mcp_version();
    
    static void Main()
    {
        // Get version
        var version = Marshal.PtrToStringAnsi(mcp_version());
        Console.WriteLine($"Version: {version}");
        
        // Create server
        uint serverId = mcp_server_create("127.0.0.1:8080");
        if (serverId == 0)
        {
            var error = Marshal.PtrToStringAnsi(mcp_last_error());
            throw new Exception($"Failed: {error}");
        }
        
        try
        {
            // Chat
            var request = JsonSerializer.Serialize(new { prompt = "Hello", agent = false });
            var responsePtr = mcp_server_chat(serverId, request, (nuint)request.Length);
            var response = Marshal.PtrToStringAnsi(responsePtr);
            mcp_string_free(responsePtr);
            
            Console.WriteLine($"Response: {response}");
        }
        finally
        {
            mcp_server_stop(serverId);
        }
    }
}
```

### Java (JNA)

```java
import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;

public class Main {
    public interface AntikytheraLibrary extends Library {
        AntikytheraLibrary INSTANCE = Native.load("antikythera_sdk", AntikytheraLibrary.class);
        
        int mcp_server_create(String addr);
        Pointer mcp_server_chat(int serverId, String requestJson, int requestLen);
        void mcp_string_free(Pointer ptr);
        int mcp_server_stop(int serverId);
        Pointer mcp_last_error();
    }
    
    public static void main(String[] args) {
        int serverId = AntikytheraLibrary.INSTANCE.mcp_server_create("127.0.0.1:8080");
        if (serverId == 0) {
            Pointer error = AntikytheraLibrary.INSTANCE.mcp_last_error();
            throw new RuntimeException("Failed: " + error.getString(0));
        }
        
        try {
            String request = "{\"prompt\": \"Hello\", \"agent\": false}";
            Pointer response = AntikytheraLibrary.INSTANCE.mcp_server_chat(serverId, request, request.length());
            String responseStr = response.getString(0);
            AntikytheraLibrary.INSTANCE.mcp_string_free(response);
            
            System.out.println("Response: " + responseStr);
        } finally {
            AntikytheraLibrary.INSTANCE.mcp_server_stop(serverId);
        }
    }
}
```

---

## Unit Tests

The FFI module includes 30+ comprehensive unit tests covering:

### Server Lifecycle Tests
- ✅ Create and stop server
- ✅ Create with CORS
- ✅ Invalid address handling
- ✅ Multiple servers
- ✅ Server count tracking
- ✅ Server list functionality

### Chat Tests
- ✅ Simple chat request
- ✅ Agent mode chat
- ✅ Chat with session ID
- ✅ Chat with attachments
- ✅ Chat with system prompt
- ✅ Invalid server handling
- ✅ Invalid JSON handling
- ✅ Empty prompt handling
- ✅ Concurrent operations

### Configuration Tests
- ✅ Get config
- ✅ Reload config
- ✅ Update config
- ✅ Partial config update
- ✅ Invalid JSON handling

### Error Handling Tests
- ✅ Error message retrieval
- ✅ Error clearing
- ✅ Null pointer handling

### Memory Management Tests
- ✅ String allocation and freeing
- ✅ Null pointer safety

### Run Tests

```bash
cargo test -p antikythera-sdk --features ffi --lib ffi
```

---

## Error Codes

| Code | Meaning |
|------|----------|
| Server ID = 0 | Failed to create server |
| Return NULL | Operation failed |
| Empty string | No data available |

Always check `mcp_last_error()` after failures!

---

## Thread Safety

- ✅ Multiple servers can run concurrently
- ✅ Chat requests are thread-safe
- ⚠️ `mcp_last_error()` is thread-local
- ✅ Server registry is mutex-protected

---

## Performance

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Server create | ~1ms | 1000+/sec |
| Chat request | ~50-200ms | Depends on LLM |
| Get config | <1ms | 10000+/sec |
| Memory alloc | <1μs | 1M+/sec |

---

## Best Practices

1. **Always free strings**: Call `mcp_string_free()` on every returned string
2. **Check errors**: Always check return values and call `mcp_last_error()`
3. **Reuse servers**: Don't create/destroy servers frequently
4. **Handle NULL**: Check for NULL pointers before dereferencing
5. **Thread safety**: Use proper synchronization for shared state
