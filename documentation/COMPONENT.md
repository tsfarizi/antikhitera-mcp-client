# Component

This document explains the WASM component model used by the project documentation.

## Overview

The component model keeps agent logic inside the component and pushes environment-specific I/O into the host.

## Component view

```mermaid
flowchart LR
    HOST[Host application] --> IMPORTS[Host imports]
    IMPORTS --> COMPONENT[WASM component]
    COMPONENT --> EXPORTS[Component exports]
    EXPORTS --> HOST
```

## Responsibility model

```mermaid
flowchart TD
    subgraph Host
        CALL_LLM[Call LLM]
        RUN_TOOLS[Run tools]
        STORE_STATE[Persist state]
        LOG[Handle logging]
    end

    subgraph Component
        PLAN[Agent logic]
        PARSE[Parse responses]
        STEP[Track steps]
    end

    CALL_LLM --> PLAN
    RUN_TOOLS --> PLAN
    STORE_STATE --> STEP
    PLAN --> LOG
```

## Why this design is useful

| Benefit | Explanation |
|:--------|:------------|
| Portability | The same component can run in different hosts |
| Separation of concerns | Runtime integration stays outside the component |
| Better host control | Providers, tools, and storage remain host-managed |

## Related documents

- [`WASM_AGENT.md`](WASM_AGENT.md)
- [`BUILD.md`](BUILD.md)
