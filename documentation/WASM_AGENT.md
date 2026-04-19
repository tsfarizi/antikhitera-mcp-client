# WASM Agent

This document describes the WASM-side agent model at a high level.

## Overview

The WASM agent focuses on agent logic and response processing, while the host side handles external I/O.

## Responsibility split

```mermaid
flowchart LR
    HOST[Host] --> LLM[LLM calls]
    HOST --> TOOLS[Tool execution]
    HOST --> STATE[Persistence]
    WASM[WASM agent] --> PARSE[Parse model output]
    WASM --> PLAN[Track state and next step]
    LLM --> WASM
    TOOLS --> WASM
    STATE --> WASM
```

## Why this split matters

| WASM side | Host side |
|:----------|:----------|
| Agent reasoning loop | External API calls |
| Response parsing | Tool execution |
| Step management | Persistence and environment integration |

## Benefits

- Keeps the WASM side smaller and more portable
- Lets the host choose provider and infrastructure strategy
- Avoids embedding every I/O concern into the component itself

## Message and session flow

The intended host/WASM exchange is:

1. The first host request may contain only plain user text.
2. The framework creates or continues a `session_id` and assembles the internal message history.
3. The framework emits a prepared message list for the host to send to the LLM.
4. The host calls the provider API and may return either:
    - plain text, or
    - a structured assistant message already shaped to match framework expectations.
5. The framework records that assistant turn into history so the next prepared request remains tied to the same WASM-side context.

This allows the host to own provider-specific payload shaping while the framework owns conversation continuity, step tracking, and response interpretation.

## Related documents

- [`COMPONENT.md`](COMPONENT.md)
- [`JSON_SCHEMA.md`](JSON_SCHEMA.md)
