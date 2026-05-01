# Architecture

This document gives a current high-level view of how the main crates interact.

## System view

```mermaid
flowchart TD
    USER[User or host application]
    CLI[antikythera-cli]
    SDK[antikythera-sdk]
    CORE[antikythera-core]
    SESSION[antikythera-session]
    LOG[antikythera-log]
    MCP[MCP servers]
    LLM[LLM providers]

    USER --> CLI
    USER --> SDK
    CLI --> CORE
    CLI --> SDK
    CLI -.->|Debug History| SESSION
    SDK --> CORE
    SDK --> SESSION
    SDK --> LOG
    CORE --> LOG
    CORE --> SESSION
    CORE --> MCP
    CORE --> LLM
```

## Core Principles

- **Single Source of Truth for Session:** `antikythera-session` owns the conversational data model (`Message`, `MessageRole`, `MessagePart`) and provides the thread-safe `SessionManager`. Both `CORE` (for actual context injection) and `CLI` (for debug persistence) utilize this unified model.
- **Stateless Tooling:** `CORE` orchestrates LLM dispatch, agent loops, and MCP tools, delegating long-term conversational memory to `SESSION`.
- **FFI & Portability:** `SDK` exposes `SESSION` and `LOG` components over safe FFI boundaries, allowing host languages (e.g. Node.js, Python) to import/export chat histories easily using the `Postcard` binary format.

## Request flow

```mermaid
sequenceDiagram
    participant User
    participant Surface as CLI or SDK
    participant Core as antikythera-core
    participant Session as antikythera-session
    participant Provider as LLM provider
    participant Server as MCP server

    User->>Surface: Send prompt or task
    Surface->>Core: Build request
    Core->>Session: Load previous history & metadata
    Core->>Provider: Generate response
    Provider-->>Core: Model output
    Core->>Server: Tool call if needed
    Server-->>Core: Tool result
    Core->>Session: Sync usage, tokens & append messages
    Core-->>Surface: Final response
    Surface-->>User: Output
```

## Crate reading order

- `antikythera-session` defines the data models and handles state retention/snapshots.
- `antikythera-core` is the main place to understand runtime behavior, orchestration, and context pruning.
- `antikythera-sdk` is the best view of the exported integration surface (FFI boundaries).
- `antikythera-cli` is the user-facing binary layer over core.
