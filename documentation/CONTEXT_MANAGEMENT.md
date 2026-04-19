# Advanced Context Management

## Overview

Antikythera's advanced context management system ensures **long-running agent conversations remain within LLM token limits** without losing critical information.

The system balances three competing goals:
1. **Stay within token budgets** — never exceed the model's context window
2. **Retain important history** — keep early messages (system prompt, task definition) + recent history
3. **Summarize when needed** — use rolling summarization to compress old conversation into a single summary

---

## Key Components

### 1. Token Estimation (`TokenEstimator`)

Fast approximation without external tokenizer:
```rust
pub struct TokenEstimator {
    // Assumes 1 token ≈ 4 characters
    // No tokenizer dependency — safe for WASM
}

let estimator = TokenEstimator::new();
let tokens = estimator.estimate_tokens("Hello, world!"); // ≈ 3 tokens
```

### 2. Truncation Strategies

Two strategies for removing messages when history exceeds `max_history_messages`:

#### `KeepNewest`
Removes oldest messages first; retains most recent history.
```json
{
  "truncation_strategy": "keep_newest",
  "max_history_messages": 10
}
```
**When to use:** Short-term conversations, sessions prioritizing recency.

#### `KeepBalanced` ✨
Retains **head** (first N/2 messages) and **tail** (last N/2 messages); removes middle messages to create space for a **rolling summary**.
```json
{
  "truncation_strategy": "keep_balanced",
  "max_history_messages": 10  // Keep 5 oldest + 5 newest
}
```
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

## Per-Provider / Per-Model Policy Override

Different LLM providers have different context windows:

| Provider | Typical Context | Recommended Policy |
|----------|-----------------|-------------------|
| OpenAI GPT-4 Turbo | 128K tokens | `max_history: 100`, `summarize_after: 80` |
| Claude 3 | 200K tokens | `max_history: 150`, `summarize_after: 120` |
| Gemini Pro | 32K tokens | `max_history: 20`, `summarize_after: 15` |
| Local Ollama | Varies (check model) | Conservative: `max_history: 10` |

### Activating Per-Provider Override

**Native Lane (CLI):**
```bash
antikythera --mode stdio \
  --provider gemini-pro \
  --model "models/gemini-pro" \
  --context-policy-file custom-policy.json
```

**WASM Component Lane:**
```rust
// In host code, before calling prepare_user_turn():
set_context_policy(serde_json::json!({
    "provider": "gemini-pro",
    "model": "models/gemini-pro",
    "policy": {
        "max_history_messages": 20,
        "summarize_after_messages": 15,
        "summary_max_chars": 300,
        "truncation_strategy": "keep_balanced"
    }
}).to_string())?;

// Next prepare_user_turn() call will use this policy for matching provider+model
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

### WASM Component with Per-Agent Policy

```rust
// Host code (Python/Go/Node.js calling the WASM component)

// Initialize with default policy
let session_id = instance.init(config_json);

// For a specific agent, override policy to be more conservative
set_context_policy(serde_json::json!({
    "provider": "gemini-pro",
    "model": "models/gemini-pro",
    "policy": {
        "max_history_messages": 10,
        "summarize_after_messages": 8,
        "summary_max_chars": 200,
        "truncation_strategy": "keep_balanced"
    }
}).to_string());

// All subsequent prepare_user_turn() calls for this session
// will use the above policy for gemini-pro
```

---

## Testing Context Management

See `tests/sdk/wasm_agent/runner_tests.rs`:

**Test 5:** `set_context_policy_provider_override_applied_on_matching_provider_model`
- Verifies per-provider/model policy override works
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
