# JSON Schema Validation

Sistem untuk memaksa LLM menjawab dengan format JSON yang konsisten dan tervalidasi.

## Overview

Ketika `format_is_json = true`, sistem akan:
1. **Menambahkan prompt template** yang memaksa LLM output JSON
2. **Mendefinisikan schema** dengan tipe data Rust (nested structures supported)
3. **Memvalidasi response** terhadap schema
4. **Auto-retry** dengan feedback error jika validasi gagal

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  LLM Request Pipeline                               │
│                                                     │
│  1. Main prompt                                     │
│  2. + Schema prompt (jika format_is_json = true)   │
│     ├─ Schema definition                            │
│     ├─ Required structure                           │
│     └─ Example JSON                                 │
│                                                     │
│  3. LLM Response                                    │
│     ├─ Validate against schema                      │
│     ├─ If valid → return                            │
│     └─ If invalid → auto-retry with error feedback  │
└─────────────────────────────────────────────────────┘
```

## Tipe Data yang Didukung

### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `String` | Text value | `"hello"` |
| `Integer` | Whole number | `42` |
| `Float` | Decimal number | `3.14` |
| `Boolean` | True/false | `true` |

### Complex Types

| Type | Description | Example |
|------|-------------|---------|
| `Array` | List of items | `["a", "b", "c"]` |
| `Object` | Nested fields | `{"name": "...", "age": 25}` |

### Nested Structures

Schema mendukung nested objects dan arrays di dalam objects:

```json
{
  "type": "object",
  "fields": {
    "user": {
      "type": "object",
      "fields": {
        "name": { "type": "string" },
        "age": { "type": "integer" },
        "roles": {
          "type": "array",
          "items": {
            "type": "object",
            "fields": {
              "name": { "type": "string" },
              "permissions": {
                "type": "array",
                "items": { "type": "string" }
              }
            }
          }
        }
      }
    }
  }
}
```

## CLI Usage

### Register Schema

```bash
# Register schema from JSON file
antikythera-config json-schema register my_schema schema.json

# Register inline
antikythera-config json-schema register user_response '{
  "name": "UserResponse",
  "type": "object",
  "fields": {
    "user_id": { "type": "string", "required": true },
    "status": { "type": "string", "required": true },
    "data": {
      "type": "object",
      "fields": {
        "name": { "type": "string" },
        "age": { "type": "integer" }
      }
    }
  }
}'
```

### Validate Response

```bash
# Validate JSON against schema
antikythera-config json-schema validate my_schema response.json

# With auto-retry (max 3 retries)
antikythera-config json-schema validate my_schema response.json --max-retries 3
```

### Generate Example

```bash
# Generate example JSON from schema
antikythera-config json-schema example my_schema
```

## FFI Usage

### Python Example

```python
import ctypes
import json

lib = ctypes.CDLL("./libantikythera_sdk.so")

# 1. Define schema
schema = {
    "name": "UserResponse",
    "type": "object",
    "fields": {
        "user_id": {
            "type": "string",
            "required": True,
            "description": "Unique user identifier"
        },
        "status": {
            "type": "string",
            "required": True,
            "description": "User status (active/inactive)"
        },
        "profile": {
            "type": "object",
            "fields": {
                "name": { "type": "string", "required": True },
                "age": { "type": "integer", "required": False },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        }
    }
}

# 2. Register schema
result = json.loads(lib.mcp_json_schema_register(
    b"user_response",
    json.dumps(schema).encode()
).decode())
print(f"Schema registered: {result['success']}")
print(f"Prompt instruction: {result['prompt_instruction']}")

# 3. Get schema prompt to append to LLM prompt
schema_prompt = lib.mcp_json_schema_prompt(b"user_response").decode()
full_prompt = f"{main_prompt}\n\n{schema_prompt}"

# 4. Send to LLM and get response
llm_response = send_to_llm(full_prompt)

# 5. Validate response
validation = json.loads(lib.mcp_json_validate(
    b"user_response",
    llm_response.encode(),
    3  # max retries
).decode())

if validation['valid']:
    print("✓ Response is valid JSON")
    print(f"Data: {validation['json']}")
else:
    print(f"✗ Validation failed: {validation['error']}")
    print(f"Retries attempted: {validation['retry_count']}")
```

### Node.js Example

```javascript
const ffi = require('ffi-napi');
const ref = require('ref-napi');

const lib = ffi.Library('./libantikythera_sdk', {
  'mcp_json_schema_register': ['pointer', ['string', 'string']],
  'mcp_json_schema_prompt': ['pointer', ['string']],
  'mcp_json_validate': ['pointer', ['string', 'string', 'uint32']],
  'mcp_json_retry_init': ['pointer', ['string', 'uint32']],
  'mcp_json_retry_prompt': ['pointer', ['string', 'string', 'string']],
});

function readCString(ptr) { return ref.readCString(ptr); }

// Register schema
const schema = {
  name: "AnalysisResult",
  type: "object",
  fields: {
    summary: { type: "string", required: true },
    confidence: { type: "float", required: true },
    findings: {
      type: "array",
      items: {
        type: "object",
        fields: {
          category: { type: "string" },
          score: { type: "integer" }
        }
      }
    }
  }
};

lib.mcp_json_schema_register("analysis", JSON.stringify(schema));

// Get schema prompt
const schemaPrompt = readCString(lib.mcp_json_schema_prompt("analysis"));

// Validate LLM response
const result = JSON.parse(readCString(
  lib.mcp_json_validate("analysis", llmResponse, 3)
));

if (!result.valid) {
  console.log(`Validation failed: ${result.error}`);
}
```

## FFI Function Reference

### Schema Management

#### `mcp_json_schema_register`

Register a new JSON schema.

```c
char* mcp_json_schema_register(const char* schema_name, const char* schema_json);
```

**Returns:**
```json
{
  "success": true,
  "schema_name": "user_response",
  "prompt_instruction": "You must respond with a JSON object matching this schema..."
}
```

#### `mcp_json_schema_get`

Get registered schema definition.

#### `mcp_json_schema_list`

List all registered schema names.

#### `mcp_json_schema_remove`

Remove a schema by name.

#### `mcp_json_schema_example`

Generate example JSON from schema.

### Validation

#### `mcp_json_validate`

Validate JSON response against schema with auto-retry.

```c
char* mcp_json_validate(const char* schema_name, const char* json_response, uint32_t max_retries);
```

**Returns:**
```json
{
  "valid": true,
  "error": null,
  "retry_count": 0,
  "json": "{...validated json...}"
}
```

Or on failure:
```json
{
  "valid": false,
  "error": "Type mismatch at $.age: expected integer, got string",
  "retry_count": 3,
  "json": "{...original response...}"
}
```

#### `mcp_json_schema_prompt`

Get schema prompt instruction to append to LLM prompt.

### Retry Management

#### `mcp_json_retry_init`

Initialize retry manager for a session.

```c
char* mcp_json_retry_init(const char* session_id, uint32_t max_retries);
```

#### `mcp_json_retry_record_error`

Record validation error for tracking.

#### `mcp_json_retry_prompt`

Generate retry prompt with error history for LLM.

#### `mcp_json_retry_is_exhausted`

Check if retry attempts are exhausted.

## Schema Definition Format

### Simple Object

```json
{
  "name": "SimpleResponse",
  "type": "object",
  "fields": {
    "message": { "type": "string", "required": true },
    "code": { "type": "integer", "required": true },
    "debug": { "type": "boolean", "required": false }
  }
}
```

### Nested Object

```json
{
  "name": "NestedResponse",
  "type": "object",
  "fields": {
    "user": {
      "type": "object",
      "fields": {
        "id": { "type": "string", "required": true },
        "profile": {
          "type": "object",
          "fields": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
          }
        }
      }
    }
  }
}
```

### Array of Objects

```json
{
  "name": "ListResponse",
  "type": "object",
  "fields": {
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "fields": {
          "id": { "type": "string" },
          "name": { "type": "string" }
        }
      }
    },
    "total": { "type": "integer" }
  }
}
```

## How It Works

### 1. Schema Registration

```rust
let schema = JsonSchema {
    name: "UserResponse".to_string(),
    root_type: SchemaType::Object {
        fields: HashMap::from([
            ("user_id", SchemaField {
                field_type: SchemaType::String,
                required: true,
                description: Some("User ID".to_string()),
                example: None,
            }),
            // ... more fields
        ]),
    },
    description: Some("User response schema".to_string()),
    allow_additional: false,
};
```

### 2. Prompt Generation

Schema generates prompt instruction:
```
You must respond with a JSON object matching this schema:
Schema: UserResponse
Description: User response schema

Required structure:
object:
  user_id (required):
    (string)
  status (required):
    (string)
  profile (optional):
    object:
      name (required):
        (string)
      age (optional):
        (integer)

Example output:
{
  "user_id": "example_string",
  "status": "example_string",
  "profile": {
    "name": "example_string",
    "age": 0
  }
}

IMPORTANT: Respond with ONLY valid JSON. No explanations or markdown.
```

### 3. Validation

When LLM responds:
1. Parse JSON
2. Validate against schema type tree
3. Check required fields
4. If valid → return
5. If invalid → generate retry prompt with specific error

### 4. Auto-Retry

If validation fails:
1. Record error in retry manager
2. Generate retry prompt with:
   - Specific validation error
   - Original response
   - Schema definition
3. Send to LLM again
4. Repeat until valid or max retries

## Use Cases

### 1. Data Extraction

```json
{
  "name": "DataExtraction",
  "type": "object",
  "fields": {
    "entities": {
      "type": "array",
      "items": {
        "type": "object",
        "fields": {
          "name": { "type": "string" },
          "type": { "type": "string" },
          "confidence": { "type": "float" }
        }
      }
    },
    "summary": { "type": "string" }
  }
}
```

### 2. API Response Format

```json
{
  "name": "ApiResponse",
  "type": "object",
  "fields": {
    "status": { "type": "string", "required": true },
    "data": { "type": "object" },
    "error": { "type": "string", "required": false },
    "pagination": {
      "type": "object",
      "fields": {
        "page": { "type": "integer" },
        "per_page": { "type": "integer" },
        "total": { "type": "integer" }
      }
    }
  }
}
```

### 3. Analysis Results

```json
{
  "name": "AnalysisResult",
  "type": "object",
  "fields": {
    "overall_score": { "type": "integer" },
    "categories": {
      "type": "array",
      "items": {
        "type": "object",
        "fields": {
          "name": { "type": "string" },
          "score": { "type": "integer" },
          "feedback": { "type": "string" }
        }
      }
    },
    "recommendations": {
      "type": "array",
      "items": { "type": "string" }
    }
  }
}
```

## Best Practices

1. **Define schemas before LLM calls** - Register schema first
2. **Use schema prompt** - Append to main prompt for better compliance
3. **Set reasonable max retries** - 3-5 retries is usually enough
4. **Use specific field descriptions** - Helps LLM understand expectations
5. **Provide examples** - Add example values to schema fields
6. **Validate early** - Check response before further processing
