# Migration

This file records that the documentation structure has changed over time.

## Current structure

The repository now keeps **one README only** at the repository root:

- `README.md`

All focused guides live under `documentation/` and use uppercase direct filenames such as:

- `ARCHITECTURE.md`
- `BUILD.md`
- `CLI.md`
- `COMPONENT.md`
- `WORKSPACE.md`

## What changed

```mermaid
flowchart LR
    OLD[Mixed naming and nested README] --> NEW[Single root README plus uppercase direct docs]
```

### Before

- `documentation/README.md` existed as a second documentation index.
- Several files used long or mixed-style names such as `CLI_DOCUMENTATION.md`, `json-schema-validation.md`, and `wasm-component-host-imports.md`.

### After

- `documentation/README.md` was removed.
- Root `README.md` became the single entry point for the repository.
- Documentation filenames under `documentation/` were normalized to direct uppercase names.

## Why this changed

- To make the repository easier to scan from the root.
- To keep documentation links predictable.
- To reduce duplicate entry points and filename noise.

## Reading advice

If you are looking for current usage or structure, prefer:

1. `README.md`
2. `documentation/WORKSPACE.md`
3. `documentation/ARCHITECTURE.md`
4. `documentation/CLI.md`
5. `documentation/BUILD.md`

Use this file only as historical context.
