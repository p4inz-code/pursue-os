# PURSUE OS — Repository Structure

> **Source:** PURSUE OS master handoff (authoritative) for the intended tree. Restored 2026-08-09.
> **Category:** B — implementation documentation (kept synchronized with the actual repository).

## Intended Structure (from handoff)

```
pursue-os/
├── .github/            # workflows/, ISSUE_TEMPLATE/, PULL_REQUEST_TEMPLATE.md
├── assets/             # artwork/, branding/, icons/
├── boot/
├── build/
├── config/
├── docs/               # ai/ architecture/ core/ design/ development/
│                       # investigation/ legal/ plugins/ release/ security/ tools/
├── os/                 # base/ desktop/ networking/ packages/ security/ services/ tor/
├── packaging/          # installer/ iso/ packages/
├── plugins/            # examples/ official/
├── scripts/            # build/ development/ release/ testing/
├── src/                # ai/ browser/ case/ common/ core/ cti/ dfir/ evidence/
│                       # geoint/ graph/ infrastructure/ media/ osint/ plugins/
│                       # provenance/ reporting/ search/ security/ socmint/ terminal/
├── tests/              # e2e/ forensic/ integration/ security/ ui/ unit/
├── tools/              # cti/ development/ dfir/ geoint/ official/ osint/
├── ui/                 # browser/ case/ components/ evidence/ graph/
│                       # settings/ shell/ terminal/ themes/
└── (top-level files)   # .editorconfig, .gitignore, LICENSE, NOTICE, README.md,
                        # SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, TRADEMARKS.md
```

The handoff states some directories are intentionally empty and that no placeholder files should be created merely to make Git track empty folders.

## Current Implementation Status (2026-08-09)

- Top-level files and `docs/` are fully tracked (docs restored from the master handoff).
- **Foundation additions:**
  - `Cargo.toml` — Rust workspace root.
  - `rust-toolchain.toml` — pinned stable toolchain.
  - `crates/pursue-core` — shared primitives.
  - `crates/pursue-evidence` — evidence integrity foundation (content addressing, audit log, stores).
  - `Cargo.lock` — locked dependency graph (reproducibility).
  - `.github/workflows/ci.yml` — CI pipeline (fmt, clippy, tests on ubuntu + windows).
- **Phase 1C additions (core runtime):**
  - `crates/pursue-runtime` — core runtime foundation: configuration (TOML), structured logging, service lifecycle, and the IPC/service boundary (see `docs/development/CORE_RUNTIME.md`).
  - `docs/development/CORE_RUNTIME.md` — Phase 1C implementation guide.
- **Phase 1D additions (case core):**
  - `crates/pursue-case` — case foundation: validated `CaseId`, the `Case` model, and audited case operations (see `docs/development/CASE_FOUNDATION.md`).
  - `docs/development/CASE_FOUNDATION.md` — Phase 1D implementation boundary (case core implemented; case store layer pending).
- The handoff's top-level directories (`os/`, `packaging/`, `plugins/`, `scripts/`, `src/`, `tests/`, `tools/`, `ui/`, `assets/`, `boot/`, `build/`, `config/`) are **created during their respective implementation phases** with real content; empty versions are not preserved in Git.
