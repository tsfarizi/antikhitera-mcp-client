# Testing Conventions

This repository keeps integration and module-level test suites under the `tests/` folder.

## Structure

- Keep test entry files thin.
- Split large suites into section files under a directory with the same base name.
- Use `include!("<suite>/<part_xx>.rs")` from the suite root file.

Examples:

- `tests/log/comprehensive_logging_tests.rs`
- `tests/log/comprehensive_logging_tests/part_01.rs`

## Separation of Concerns (SoC)

- Group tests by concern (e.g., edge cases, concurrency, security, serialization).
- Keep imports and shared setup in the suite root file only.
- Keep each part file focused on one concern section.

## Readability

- Prefer short, descriptive test names.
- Keep one assertion intent per test where possible.
- Use ASCII-safe Unicode escapes (for example `\u{1f680}`) in test fixtures when portability is needed.

## File Size Guideline

- Aim to keep test files below ~300 lines.
- If a suite grows beyond that, split into additional part files.
