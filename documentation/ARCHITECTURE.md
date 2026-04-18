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

## Crate reading order

- `antikythera-core` is the main place to understand runtime behavior.
- `antikythera-sdk` is the best view of the exported integration surface.
- `antikythera-cli` is the user-facing binary layer over core.
