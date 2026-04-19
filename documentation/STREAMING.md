# Streaming (Phase 1 & Phase 2)

This document describes the streaming surface for token output, intermediate
agent events, tool-result streaming, summary streaming, buffered/unbuffered
flush policies, and client-side input chunking.

## Phase 1 — Token / Event streaming

Phase 1 adds:

- `antikythera_core::application::streaming` primitives
- Host-facing SDK options in `antikythera_sdk::agents::StreamingOptions`
- CLI adapter flag `--stream` for terminal token output

Non-streaming APIs remain unchanged.

## Phase 2 — Advanced Streaming (v0.9.9 Priority 4)

Phase 2 extends the surface without breaking Phase 1 consumers:

| New type | Purpose |
|---|---|
| `AgentEvent::ToolResult` | Streaming chunks of tool-execution output |
| `AgentEvent::Summary` | Streaming context-management summarisation chunks |
| `BufferPolicy` | `Unbuffered` vs `Buffered { flush_threshold }` flush control |
| `StreamingBuffer` | Accumulates events and signals when a batch is ready |
| `ClientInputStream` | Host-side chunked input for large payloads |
| `StreamingPhase2Options` | Opt-in config embedded in `StreamingRequest::phase2` |

## Core primitives

- `StreamingRequest` with `mode`, `include_final_response`, `max_buffered_events`, and optional `phase2: Option<StreamingPhase2Options>`
- `StreamingMode` (`token`, `event`, `mixed`)
- `AgentEvent` (`token`, `tool`, `state`, `completed`, `tool_result`, `summary`)
- `AgentEventStream` bounded FIFO event queue with `push_tool_result` / `push_summary` helpers
- `StreamingResponse` trait for provider/runtime adapters
- `InMemoryStreamingResponse` test/host collector

### Phase 1 example

```rust
use antikythera_core::{
    AgentEvent, InMemoryStreamingResponse, StreamingRequest, StreamingResponse,
};

let mut response = InMemoryStreamingResponse::new(StreamingRequest::default());
response.push_token("Hello".to_string());
response.push_event(AgentEvent::Completed);
response.set_final_response("Hello world".to_string());

let snapshot = response.snapshot();
assert_eq!(snapshot.tokens, vec!["Hello"]);
assert_eq!(snapshot.events.len(), 2);
```

### Phase 2 — streaming tool results

```rust
use antikythera_core::{
    AgentEvent, AgentEventStream, ToolEventPhase,
};

let mut stream = AgentEventStream::new();
// tool starts
stream.push_tool("search", ToolEventPhase::Started);
// intermediate result chunks
stream.push_tool_result("search", "line 1\n", false);
stream.push_tool_result("search", "line 2\n", true); // last chunk
// tool finishes
stream.push_tool("search", ToolEventPhase::Finished);
```

### Phase 2 — streaming summaries

```rust
use antikythera_core::AgentEventStream;

let mut stream = AgentEventStream::new();
stream.push_summary("First half of condensed context.", false, 0);
stream.push_summary(" Second half.", true, 12); // original_message_count=12
```

### Phase 2 — buffered flush

```rust
use antikythera_core::streaming::{AgentEvent, BufferPolicy, StreamingBuffer};

let mut buf = StreamingBuffer::new(BufferPolicy::Buffered { flush_threshold: 4 });

for i in 0..4 {
    let ready = buf.push(AgentEvent::Completed);
    if ready {
        let batch = buf.flush();
        // deliver batch to host
        assert_eq!(batch.len(), 4);
    }
}
```

### Phase 2 — client-side input chunking

```rust
use antikythera_core::streaming::ClientInputStream;

let mut input = ClientInputStream::new();
// push chunks as they arrive (e.g. from stdin or HTTP chunks)
input.push_chunk("The quick ");
input.push_chunk("brown fox");
input.complete();

assert!(input.is_complete());
let full_text = input.collect_all();
assert_eq!(full_text, "The quick brown fox");
```

### Phase 2 — opt-in options

Embed `StreamingPhase2Options` in a request to activate Phase 2 features:

```rust
use antikythera_core::streaming::{
    BufferPolicy, StreamingPhase2Options, StreamingRequest,
};

let request = StreamingRequest {
    phase2: Some(StreamingPhase2Options {
        buffer_policy: BufferPolicy::Buffered { flush_threshold: 8 },
        include_tool_results: true,
        include_summaries: false, // suppress summary events
    }),
    ..StreamingRequest::default()
};
```

When `phase2` is `None` the stream behaves exactly as Phase 1 — all events
pass through and buffering is not applied.

## SDK options

`antikythera_sdk::agents::StreamingOptions` is a JSON-friendly host surface.

Example JSON:

```json
{
  "mode": "mixed",
  "include_final_response": true,
  "max_buffered_events": 128
}
```

Validation:

- `max_buffered_events` must be `> 0` when provided

FFI helpers:

- `mcp_default_streaming_options()`
- `mcp_validate_streaming_options(options_json)`

## CLI adapter

The native CLI enables terminal token streaming when `--stream` is passed.
Default behavior is unchanged for scripts and automation.

```bash
antikythera --stream --mode stdio
```

## Compatibility

- All Phase 1 APIs remain fully backward compatible
- Phase 2 features are opt-in via `StreamingRequest::phase2`
- No breaking changes to existing public APIs


Example JSON:

```json
{
  "mode": "mixed",
  "include_final_response": true,
  "max_buffered_events": 128
}
```

Validation:

- `max_buffered_events` must be `> 0` when provided

FFI helpers:

- `mcp_default_streaming_options()`
- `mcp_validate_streaming_options(options_json)`

## CLI adapter

The native CLI now enables terminal token streaming only when `--stream` is
passed. This keeps default behavior unchanged for scripts and automation.

Example:

```bash
antikythera --stream --mode stdio
```

## Compatibility

- Existing non-streaming flows remain fully supported
- Streaming is opt-in at host/CLI level
- No breaking changes to current public APIs