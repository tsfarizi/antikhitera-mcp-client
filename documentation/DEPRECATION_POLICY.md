# Deprecation Policy (v1.0.0)

This policy defines current deprecation handling for public APIs.

## Lifecycle Flow

```mermaid
flowchart LR
    Introduce[Introduce replacement] --> Mark[Mark deprecated API]
    Mark --> Warn[Emit compile-time warning]
    Warn --> Migrate[Consumer migration]
    Migrate --> Remove[Major-version removal]
```

## Policy Rules

- A replacement API must exist before deprecation is introduced.
- Deprecated APIs must include `since` metadata and migration notes.
- Deprecated APIs remain thin delegates only.
- Removal occurs only on a major version boundary.

## Enforcement

- CI/lint gate for production targets:
  - `cargo clippy --workspace --lib --bins -- -D warnings -D deprecated`
- Backward-compatibility tests can explicitly allow deprecated paths when required.
