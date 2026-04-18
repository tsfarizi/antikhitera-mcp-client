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
    SDK --> CORE
    SDK --> SESSION
    SDK --> LOG
    CORE --> LOG
    CORE --> MCP
    CORE --> LLM
```

## Request flow

```mermaid
sequenceDiagram
    participant User
    participant Surface as CLI or SDK
    participant Core as antikythera-core
    participant Provider as LLM provider
    participant Server as MCP server

    User->>Surface: Send prompt or task
    Surface->>Core: Build request
    Core->>Provider: Generate response
    Provider-->>Core: Model output
    Core->>Server: Tool call if needed
    Server-->>Core: Tool result
    Core-->>Surface: Final response
    Surface-->>User: Output
```

## CLI-specific note

The CLI crate is architected around separate domain, infrastructure, presentation, and config areas, but the shipped `antikythera` binary is still partial at runtime. The architecture is broader than the currently exposed user experience.

## Current implications

- `antikythera-core` is the main place to understand real runtime behavior.
- `antikythera-sdk` is the best view of the exported integration surface.
- `antikythera-cli` documents intent and structure, but the main binary still exposes placeholder runtime modes.
