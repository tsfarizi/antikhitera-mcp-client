# DEPRECATION POLICY

This policy defines how deprecated APIs are introduced, communicated, and removed for the Antikythera workspace.

## Scope

This policy applies to all public Rust APIs in:

- `antikythera-core`
- `antikythera-sdk`
- `antikythera-cli`
- supporting crates exported from this workspace

## Lifecycle Rules

1. Introduce replacement first.
- A deprecated API must always have a stable replacement available in the same release window.

2. Mark deprecated APIs explicitly.
- Use `#[deprecated(since = "<version>", note = "<replacement>; scheduled removal in <major>")]`.
- Notes must include both replacement symbol and planned removal major version.

3. Keep behavior identical during grace period.
- Deprecated aliases must remain thin delegates only.
- No behavior divergence between old and replacement symbols.

4. Remove only on major release.
- Deprecated public APIs are removed only on the next major boundary.
- For current aliases introduced in `0.9.9`, planned removal is `2.0.0`.

5. Document and test migration.
- Public migration docs must list old -> new mappings.
- Tests must verify alias delegation while aliases still exist.

## CI Enforcement

To avoid new code depending on deprecated aliases:

- CI runs a dedicated lint gate on production targets:
  - `cargo clippy --workspace --lib --bins -- -D warnings -D deprecated`
- Test targets may include explicit `#[allow(deprecated)]` only when validating backward compatibility.

## Current Deprecated Alias Inventory (0.9.9)

- `antikythera_cli::config::load_config` -> `load_app_config`
- `antikythera_cli::config::save_config` -> `save_app_config`
- `antikythera_cli::infrastructure::config::load_cli_config` -> `load_app_config`
- `antikythera_cli::infrastructure::config::create_llm_provider` -> `build_llm_provider`
- `antikythera_cli::infrastructure::config::create_provider_config` -> `build_active_provider_config`

## Release Checklist (Deprecation)

Before release candidates and stable releases:

1. Verify all deprecated APIs have `since` and planned removal notes.
2. Verify no production code uses deprecated aliases.
3. Verify migration docs are up to date.
4. Verify compatibility tests still pass for supported aliases.
5. Confirm removal plan aligns with semver promise.
