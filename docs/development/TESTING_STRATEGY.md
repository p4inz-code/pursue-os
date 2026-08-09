# PURSUE OS — Testing Strategy

> **Source:** PURSUE OS master handoff (authoritative) — "Never skip tests", "validate each phase". Restored 2026-08-09.
> **Category:** B — implementation documentation (maintained with the code).

## Principles

- **Never skip tests.** Every phase is validated before it is considered complete.
- **Never claim implementation without validation.**
- Test the integrity guarantees, not just the happy path (altered-data detection, invalid input handling).
- Keep tests fast, deterministic, and runnable offline.

## Levels (current foundation)

1. **Unit tests** — in-crate tests for primitives and invariants (hashing, content addressing, audit-chain verification, store round-trips).
2. **Integration tests** — cross-crate behavior (e.g., evidence store through the `pursue-evidence` public API), including persistence/retrieval across reloads.
3. **Lint/static analysis** — `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` (warnings denied).
4. **CI** — GitHub Actions runs fmt, clippy, and `cargo test` on ubuntu-latest and windows-latest (see `.github/workflows/ci.yml`).

## Command (Git Bash)

```bash
cd "/c/Users/Admin/Desktop/Pursue-OS" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## Future Levels (later phases)

Security tests, forensic tests, e2e/UI tests, and image-build tests will be added in their respective phases (the `tests/` hierarchy exists in the intended structure).
