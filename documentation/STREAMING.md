# Streaming (v1.0.0)

This document covers the currently implemented streaming behavior.

## Streaming Pipeline

```mermaid
flowchart LR
    Input[User input] --> Request[StreamingRequest]
    Request --> Events[AgentEventStream]
    Events --> Buffer[StreamingBuffer]
    Buffer --> Flush[Buffered or unbuffered flush]
    Flush --> Output[Host or CLI output]
```

## Current Capabilities

- Token and event streaming through unified request options.
- Tool-result and summary event support.
- Buffered and unbuffered emission policies.
- Client input chunk accumulation helpers.

## Runtime Guarantees

- Backward-compatible request defaults.
- Bounded buffering controls via explicit limits.
- Deterministic event draining order.
