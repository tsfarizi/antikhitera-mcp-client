# Advanced Context Management v1 (0.9.8)

## Overview

Advanced Context Management v1 provides production-grade message history and token budget management for long-running agent conversations. It handles automatic truncation, configurable strategies, and runtime policy updates while preserving conversation quality and system prompts.

## Quick Start

### Basic Usage

```rust
use antikythera_core::application::context_management::{ContextPolicy, RuntimeContextManager};
use antikythera_core::domain::types::{ChatMessage, MessageRole};

// Create manager with default policy (KeepNewest, 50 messages max)
let manager = RuntimeContextManager::new(ContextPolicy::default());

// Apply policy to filter messages
let messages = vec![
    ChatMessage::new(MessageRole::System, "You are helpful"),
    ChatMessage::new(MessageRole::User, "Tell me about Rust"),
    ChatMessage::new(MessageRole::Assistant, "Rust is a systems programming language..."),
];

let filtered = manager.apply_policy(&messages)?;
// Result: All 3 messages retained (within default 50 message limit)
```

### Balanced Strategy for Long Conversations

```rust
let policy = ContextPolicy::new()
    .with_max_history_messages(100)
    .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.3 })
    .with_token_budget(8000);

let manager = RuntimeContextManager::new(policy);
let filtered = manager.apply_policy(&long_conversation)?;
// Result: First 30% of messages + last 70% retained, middle 40% removed
```

---

## Key API Components

### ContextPolicy

Configures how message histories are managed during long conversations.

```rust
pub struct ContextPolicy {
    /// Maximum messages to retain in history (excluding system messages)
    pub max_history_messages: usize,
    
    /// Strategy for removing messages when budget exceeded
    pub truncation_strategy: TruncationStrategy,
    
    /// Minimum system messages to always preserve
    pub min_system_messages: usize,
    
    /// Optional token budget (1 token ≈ 4 characters)
    pub token_budget: Option<usize>,
}
```

**Default Values:**
- `max_history_messages`: 50
- `truncation_strategy`: KeepNewest
- `min_system_messages`: 1
- `token_budget`: None

**Builder Pattern for Fluent Configuration:**

```rust
ContextPolicy::new()
    .with_max_history_messages(100)
    .with_token_budget(4000)
    .with_min_system_messages(2)
    .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.3 })
```

**Serialization:**

ContextPolicy is JSON-serializable, suitable for config files:

```json
{
  "max_history_messages": 100,
  "truncation_strategy": {
    "type": "keep_balanced",
    "head_ratio": 0.3
  },
  "min_system_messages": 2,
  "token_budget": 4000
}
```

### TruncationStrategy

Enum defining how messages are removed when budget is exceeded.

#### `KeepNewest` (default)

Removes oldest messages first, retaining newest history.

**When to use:**
- Short-term conversations
- Sessions prioritizing recency
- Interactive debugging sessions

**Example:**
```rust
TruncationStrategy::KeepNewest

// Input:  [msg0, msg1, msg2, msg3, msg4, msg5] (6 messages)
// Policy: max_history_messages = 3
// Result: [msg3, msg4, msg5] (oldest 3 removed)
```

#### `KeepBalanced { head_ratio: f32 }`

Retains a percentage of earliest messages plus all recent messages, removing middle messages.

**When to use:**
- Long conversations requiring conversation context
- Multi-turn agent workflows
- Scenarios requiring both historical context and recent decisions

**Parameters:**
- `head_ratio` (0.0 to 1.0): Fraction of retained messages to keep from conversation start

**Example:**
```rust
TruncationStrategy::KeepBalanced { head_ratio: 0.3 }

// Input:        [msg0, msg1, msg2, msg3, msg4, msg5, msg6, msg7, msg8, msg9] (10 messages)
// Policy:       max_history_messages = 6, head_ratio = 0.3
// Head count:   6 * 0.3 = 2 (keep msg0, msg1)
// Tail count:   6 - 2 = 4 (keep msg6, msg7, msg8, msg9)
// Result:       [msg0, msg1, msg6, msg7, msg8, msg9]
// Removed:      msg2, msg3, msg4, msg5 (middle 4 messages)
```

#### `Summarize`

(Future enhancement - currently falls back to KeepNewest)

Summarizes older messages via callback to compress history.

### RuntimeContextManager

Main API for applying context policies at runtime.

```rust
pub struct RuntimeContextManager { /* ... */ }

impl RuntimeContextManager {
    /// Create a new manager with the given policy
    pub fn new(policy: ContextPolicy) -> Self
    
    /// Update the policy at runtime
    pub fn set_policy(&self, policy: ContextPolicy) -> Result<(), String>
    
    /// Get the current policy (cloned)
    pub fn get_policy(&self) -> Result<ContextPolicy, String>
    
    /// Apply the policy to messages and return filtered result
    pub fn apply_policy(&self, messages: &[ChatMessage]) -> Result<Vec<ChatMessage>, String>
    
    /// Register a summarization callback (future use)
    pub fn set_summarization_callback(&self, callback: SummarizationFn)
}
```

**Thread Safety:**

All operations use `Arc<Mutex<T>>` internally:
- ✅ Safe to share across threads
- ✅ Safe to clone (cheap Arc clone)
- ✅ Cloned instances share the same policy state

```rust
let manager = RuntimeContextManager::new(policy);
let manager_ref = manager.clone();

// Both managers share policy state
manager.set_policy(new_policy)?;
let result = manager_ref.apply_policy(&messages)?; // Uses new_policy
```

---

## Usage Patterns

### In Agent Loop

```rust
pub async fn agent_loop(
    orchestrator: &mut MultiAgentOrchestrator,
    messages: &mut Vec<ChatMessage>,
    context_manager: RuntimeContextManager,
) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        // Apply context policy before each LLM call
        let filtered_messages = context_manager.apply_policy(messages)?;
        
        // Prepare request with filtered history
        let prepared_turn = prepare_user_turn(&filtered_messages)?;
        
        // Run agent orchestrator
        let result = orchestrator.dispatch_task(&prepared_turn).await?;
        
        // Append response to full history (not filtered)
        messages.push(ChatMessage::new(MessageRole::Assistant, result.clone()));
        
        // Check termination condition
        if result.contains("Done") || messages.len() > 1000 {
            return Ok(result);
        }
    }
}
```

### Multi-Agent with Different Policies

```rust
// User-facing agent: stricter budget for fast responses
let user_manager = RuntimeContextManager::new(
    ContextPolicy::new()
        .with_max_history_messages(30)
        .with_token_budget(2000)
);

// Backend processing: larger budget for analysis
let backend_manager = RuntimeContextManager::new(
    ContextPolicy::new()
        .with_max_history_messages(200)
        .with_token_budget(16000)
        .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.2 })
);

// Apply independently
let user_filtered = user_manager.apply_policy(&conversation)?;
let backend_filtered = backend_manager.apply_policy(&conversation)?;
```

### Runtime Policy Adjustment

```rust
let manager = RuntimeContextManager::new(ContextPolicy::default());

// Start with default policy (50 messages, KeepNewest)
let filtered1 = manager.apply_policy(&messages)?;

// After detecting long conversation, switch to balanced strategy
if messages.len() > 100 {
    let new_policy = ContextPolicy::new()
        .with_max_history_messages(100)
        .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.25 });
    
    manager.set_policy(new_policy)?;
}

// Next apply_policy uses new strategy
let filtered2 = manager.apply_policy(&updated_messages)?;
```

---

## Design Principles

### Token Estimation

Uses a fast heuristic: **1 token ≈ 4 characters**

**Trade-offs:**
- ✅ Portable (no dependencies)
- ✅ WASM-safe
- ✅ O(n) complexity
- ⚠️ Approximate (±5-15% error on English text)

**For production use:** Consider integrating actual tokenizers (e.g., `tiktoken`, `SentencePiece`) and replacing the heuristic with real token counts.

### System Message Preservation

System messages are preserved because they define agent role and behavior. Removing them would break agent consistency. The system:
1. Always keeps system messages separate
2. Respects `min_system_messages` count
3. Never removes system messages during truncation

### Policy Composition

Policies are applied in sequence:
1. **Message count limit** — enforce `max_history_messages`
2. **System message minimum** — preserve `min_system_messages`
3. **Token budget** — apply token-level cutoff if configured

Example with all three:
```
Input messages: 150 total (10 system + 140 user/assistant)
max_history_messages: 60
min_system_messages: 3
token_budget: 8000

Step 1: Truncate to 60 messages total
Step 2: Preserve at least 3 system messages
Step 3: Further trim if token count > 8000
Result: ~55-60 messages, ≤ 8000 tokens, ≥ 3 system messages
```

---

## Performance Characteristics

| Operation | Complexity | Typical Time |
|-----------|-----------|---|
| `apply_policy()` | O(n log n) worst, O(n) typical | 1-10ms for 100 messages |
| `set_policy()` | O(1) | <1ms |
| `clone()` | O(1) | <1µs (Arc clone) |
| Token estimation | O(n) | 0.1-1ms for 100 messages |

**Memory:** Clones all messages during apply_policy(). For very large histories (>10K messages), consider streaming or incremental filtering.

---

## Testing

**Unit Tests (10 total):**

✅ Policy defaults and builder pattern (4 tests)
- Default values sensible
- Fluent builder works
- Serialization roundtrips
- Enum defaults correct

✅ System message preservation (1 test)
- System messages never removed

✅ Message count limits (1 test)
- Respects max_history_messages

✅ Truncation strategies (2 tests)
- KeepNewest removes oldest first
- KeepBalanced retains head and tail

✅ Token budget (1 test)
- Enforces token limit

✅ Thread safety (1 test)
- Cloning works safely

Run tests:
```bash
cargo test -p antikythera-core context_management
```

---

## Future Enhancements (v0.9.9+)

- **Actual Summarization** — Implement `Summarize` strategy with LLM-based compression
- **Importance Scoring** — Add message importance ranking to preserve high-value exchanges
- **Adaptive Strategies** — Auto-adjust truncation based on conversation quality metrics
- **Telemetry** — Export metrics on truncation frequency, patterns
- **Multi-language** — Improve token estimation for non-English text
- **Streaming** — Support incremental filtering for large histories

---

## Backward Compatibility

✅ **Fully additive feature set**

- New module imported explicitly
- No changes to existing ChatMessage API
- New managers are opt-in
- Existing code paths unaffected

Migration path for applications already managing context:
```rust
// Old way (manual filtering)
let filtered = messages.iter()
    .filter(|m| messages_to_keep(m))
    .collect();

// New way (policy-driven)
let manager = RuntimeContextManager::new(policy);
let filtered = manager.apply_policy(&messages)?;
```

---

## References

- API documentation: `antikythera_core::application::context_management`
- Module docs: `cargo doc --open` then navigate to `antikythera_core::application`
- Integration: See `antikythera-core` test suite for agent integration examples
# Advanced Context Management v1 (0.9.8)

## Overview

Antikythera's Advanced Context Management v1 ensures **long-running agent conversations remain within LLM token limits** without losing critical information through configurable truncation strategies and runtime policy management.

The system balances three competing goals:
1. **Stay within token budgets** — never exceed configured limits
2. **Retain important history** — preserve system prompts and recent exchanges
3. **Support multiple strategies** — enable KeepNewest, KeepBalanced, or future summarization approaches

---

## Quick Start

### Basic Usage

```rust
use antikythera_core::application::context_management::{ContextPolicy, RuntimeContextManager, TruncationStrategy};
use antikythera_core::domain::types::{ChatMessage, MessageRole};

// Create manager with default policy
let manager = RuntimeContextManager::new(ContextPolicy::default());

// Apply policy to filter messages
let messages = vec![
    ChatMessage::new(MessageRole::System, "You are helpful"),
    ChatMessage::new(MessageRole::User, "Hello"),
    // ... more messages
];

let filtered = manager.apply_policy(&messages)?;
```

### Balanced Strategy for Long Conversations

```rust
let policy = ContextPolicy::new()
    .with_max_history_messages(100)
    .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.3 })
    .with_token_budget(8000);

let manager = RuntimeContextManager::new(policy);
let filtered = manager.apply_policy(&messages)?;
```

---

## Key API Components

### ContextPolicy

Configuration for how message histories are managed:

```rust
pub struct ContextPolicy {
    /// Maximum messages to retain (excluding system messages)
    pub max_history_messages: usize,
    
    /// Strategy for removing messages
    pub truncation_strategy: TruncationStrategy,
    
    /// Minimum system messages to always preserve
    pub min_system_messages: usize,
    
    /// Optional token budget (1 token ≈ 4 characters)
    pub token_budget: Option<usize>,
}
```

**Defaults:**
- `max_history_messages`: 50
- `truncation_strategy`: KeepNewest
- `min_system_messages`: 1
- `token_budget`: None

**Builder Pattern:**

```rust
ContextPolicy::new()
    .with_max_history_messages(100)
    .with_token_budget(4000)
    .with_truncation_strategy(TruncationStrategy::KeepBalanced { head_ratio: 0.3 })
```

### TruncationStrategy

Enum defining how messages are removed when budget is exceeded:

#### `KeepNewest` (default)
Removes oldest messages first, retaining newest history.

**Use case:** Short-term conversations, sessions prioritizing recency.

```rust
TruncationStrategy::KeepNewest
```

**Example behavior:**
- History: `[msg0, msg1, msg2, msg3, msg4]`
- Max: 3 messages
- Result: `[msg2, msg3, msg4]` (oldest removed first)
**When to use:** Long conversations, multi-turn agent workflows, scenarios requiring both historical context and recent decisions.

**Example:**
- **Before truncation:** [msg0, msg1, msg2, msg3, msg4, msg5, msg6, msg7, msg8, msg9, msg10, msg11]
- **Rolling summary created:** "Earlier, user asked about database schema; we discussed indexing strategy..."
- **After truncation:** [msg0, msg1, msg2, **SUMMARY**, msg9, msg10, msg11]
- **Messages now:** 3 (head) + 1 (summary) + 3 (tail) = 7 messages ✓ within limit

---

### 3. Context Policy

Define when and how to manage context:

```json
{
  "max_history_messages": 20,
  "summarize_after_messages": 15,
  "summary_max_chars": 500,
  "truncation_strategy": "keep_balanced"
}
```

| Field | Meaning |
|-------|---------|
| `max_history_messages` | Hard limit: never keep more than this many messages |
| `summarize_after_messages` | Soft trigger: create rolling summary when history exceeds this |
| `summary_max_chars` | Cap on summary length to prevent it from consuming space |
| `truncation_strategy` | Which strategy to apply: `keep_newest` or `keep_balanced` |

---

## Runtime Policy Update

Context policy is now managed as a runtime-level policy for the active MCP client workflow.

Use `set_context_policy` to update the default policy used on subsequent turns:

```rust
set_context_policy(serde_json::json!({
  "policy": {
    "max_history_messages": 20,
    "summarize_after_messages": 15,
    "summary_max_chars": 300,
    "truncation_strategy": "keep_balanced"
  }
}).to_string())?;

// Next prepare_user_turn() call will use this updated default policy.
```

---

## Rolling Summarization

When `summarize_after_messages` is reached, the framework:

1. **Collects messages to summarize** — all messages except system prompt and tail (last N/2)
2. **Builds summarization prompt** — asks LLM to compress the conversation
3. **Creates summary message** — role: `assistant`, content: "SUMMARY: [compressed text]"
4. **Replaces old messages** — removes summarized messages, inserts summary
5. **Continues conversation** — framework resumes with truncated + summarized history

**Result:** No loss of critical information, but old turns are compressed into a single summary message.

---

## Integration Examples

### Native CLI with Long Conversation

```bash
# 1. Start a chat session with KeepBalanced policy
antikythera --mode stdio

# Framework config (app.pc):
# {
#   "max_steps": 100,
#   "context_policy": {
#     "max_history_messages": 20,
#     "summarize_after_messages": 15,
#     "summary_max_chars": 400,
#     "truncation_strategy": "keep_balanced"
#   }
# }

# 2. As conversation grows past 15 messages:
# - Framework calls summarize_llm_context()
# - Creates a single SUMMARY message
# - Truncates to: [system] [SUMMARY] [recent 10]
# - Continues chat

# 3. User keeps chatting — framework automatically manages context
```

### WASM Component with Runtime Policy Update

```rust
// Host code (Python/Go/Node.js calling the WASM component)

// Initialize with default policy
let session_id = instance.init(config_json);

// Update runtime default policy to be more conservative
set_context_policy(serde_json::json!({
    "policy": {
        "max_history_messages": 10,
        "summarize_after_messages": 8,
        "summary_max_chars": 200,
        "truncation_strategy": "keep_balanced"
    }
}).to_string());

// All subsequent prepare_user_turn() calls will use the updated policy
```

---

## Testing Context Management

See `tests/sdk/wasm_agent/runner_tests.rs`:

**Test 5:** `set_context_policy_applies_global_policy_on_next_turn`
- Verifies runtime global policy update works
- Confirms summarization is triggered on policy threshold

**Test 6:** `keep_balanced_truncation_retains_head_and_tail`
- Verifies KeepBalanced strategy retains first and last messages
- Confirms rolling summary is created
- Validates history length stays within limit

```bash
cargo test -p antikythera-tests --test wasm_agent_runner_tests keep_balanced
cargo test -p antikythera-tests --test wasm_agent_runner_tests set_context_policy
```

---

## Advanced Tuning

### For Long Research Sessions

```json
{
  "max_history_messages": 50,
  "summarize_after_messages": 40,
  "summary_max_chars": 1000,
  "truncation_strategy": "keep_balanced"
}
```
✓ Keeps lots of context (40-50 messages)  
✓ Allows large summaries (up to 1000 chars)  
✓ Retains both old decisions and recent steps

### For Fast / Cost-Conscious Sessions

```json
{
  "max_history_messages": 5,
  "summarize_after_messages": 3,
  "summary_max_chars": 100,
  "truncation_strategy": "keep_newest"
}
```
✓ Minimal token usage  
✓ Only recent history kept  
✓ KeepNewest removes old messages entirely

### For Competitive Scenarios

```json
{
  "max_history_messages": 15,
  "summarize_after_messages": 10,
  "summary_max_chars": 300,
  "truncation_strategy": "keep_balanced"
}
```
✓ Balanced tradeoff  
✓ Retains context from start and recent  
✓ Summaries stay concise but informative

---

## Related Documentation

- [`documentation/RESILIENCE.md`](RESILIENCE.md) — Retry, timeout, and health tracking
- [`documentation/COMPONENT.md`](COMPONENT.md) — WASM component host integration
- [`tests/sdk/wasm_agent/runner_tests.rs`](../../tests/sdk/wasm_agent/runner_tests.rs) — Integration tests
