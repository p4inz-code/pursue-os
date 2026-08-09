# PURSUE OS — V1 Release Plan

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative phase plan.

## Phase Sequence (from handoff)

- **Phase 1A — Technical architecture lock:** base Linux distribution, build system, package strategy, desktop environment, language/runtime choices, core service architecture, UI framework, data/storage architecture, security boundaries, test framework, CI strategy, ISO/build architecture. Research as needed; ask the user only for owner-level choices.
- **Phase 1B — Minimal bootable PURSUE base:** reproducible development/build environment; eventually a bootable minimal PURSUE image. Do not implement every feature at once.
- **Phase 1C — Core runtime:** core services, configuration, logging, secure storage foundation, IPC/service boundaries, update/package foundations, test harness.
- **Phase 1D — Case/evidence foundation:** case model, evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata.
- **Phase 1E — Terminal foundation** (flagship phase).
- **Phase 1F — Browser + Tor foundation** (dedicated major phase; Tor integration in detail together with the terminal).

## Stop Conditions (do not begin before their phases)

- Full Investigation Terminal
- Full Investigation Browser
- Tor integration (as a completed feature)
- Large OSINT tool collection
- Advanced DFIR tooling
- Graph UI / reporting UI
- Plugin marketplace
- Advanced AI
- Production release / final ISO

## Current Progress

- Phase 1A: technical architecture lock — in progress (see DECISION_RECORD Part B and the foundation implementation).
- Phases 1B–1F: not started.
