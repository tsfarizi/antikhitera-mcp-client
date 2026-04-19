# Streaming (Phase 1)

This document describes the phase-1 streaming surface for token and
intermediate agent events.

## Scope

Phase-1 streaming adds:

- `antikythera_core::application::streaming` primitives
- host-facing SDK options in `antikythera_sdk::agents::StreamingOptions`
- CLI adapter flag `--stream` for terminal token output

Non-streaming APIs remain unchanged.

## Core primitives

The core module provides these types:

- `StreamingRequest` with `mode`, `include_final_response`, and optional event buffer bound
- `StreamingMode` (`token`, `event`, `mixed`)
- `AgentEvent` (`token`, `tool`, `state`, `completed`)
- `AgentEventStream` bounded FIFO event queue
- `StreamingResponse` trait for provider/runtime adapters
- `InMemoryStreamingResponse` test/host collector

Example:

```rust
use antikythera_core::{
    AgentEvent,
    InMemoryStreamingResponse,
    StreamingRequest,
    StreamingResponse,
};

let mut response = InMemoryStreamingResponse::new(StreamingRequest::default());
response.push_token("Hello".to_string());
response.push_event(AgentEvent::Completed);
response.set_final_response("Hello world".to_string());

let snapshot = response.snapshot();
assert_eq!(snapshot.tokens, vec!["Hello"]);
assert_eq!(snapshot.events.len(), 2);
```

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