# PURSUE OS — Development Guidelines

> **Source:** PURSUE OS master handoff (authoritative) — workflow rules. Restored 2026-08-09.
> **Category:** B — implementation documentation (agent-writable).

## Terminal Workflow (Git Bash only)

- **Always use Git Bash.** Never PowerShell, never CMD.
- Repository commands begin with: `cd "/c/Users/Admin/Desktop/Pursue-OS" &&`
- Prefer combined Git Bash command blocks.
- User runs command blocks manually, usually one block at a time; do not assume a command was executed unless confirmed.

## Implementing

1. **Inspect first.** Check git state and existing code before changing anything.
2. **Make a focused change.** Minimal, reversible, deliberate.
3. **Test.** Never skip tests; validate each phase.
4. **Inspect the diff.** `git diff --stat`, `git diff --check`.
5. **Commit logically.** Logical commits, not one giant unexplained commit; do not blindly commit unreviewed output.
6. **Push when appropriate.**

## Document Ownership Model

- **Category A — authoritative project decisions** (product identity, principles, V1 scope, security principles, AI boundaries, licensing, branding, major architecture boundaries, evidence/provenance principles, plugin security model, non-goals): do not casually rewrite; preserve locked decisions.
- **Category B — implementation documentation** (module docs, API docs, build docs, testing docs, implementation notes, subsystem details, tool integration docs, developer docs): agent-writable as implementation progresses.

## Principles

- Correctness over rushing.
- No missing requirements; strong validation; explicit status.
- Minimal unnecessary questions; automatic technical decisions where user preference is not required.
- Research when a technical decision genuinely benefits from current information.
- Do not assume facts when they can be checked.
- No unnecessary overengineering.

## Toolchain (current foundation)

- Rust workspace (cargo) — see REPOSITORY_STRUCTURE.md and TESTING_STRATEGY.md.
- Validation commands (Git Bash):

```bash
cd "/c/Users/Admin/Desktop/Pursue-OS" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
