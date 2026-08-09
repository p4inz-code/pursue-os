# PURSUE V1 — Scope

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative project decisions.

## In Scope (V1)

- **Technical architecture lock (Phase 1A)** — base Linux distribution, build system, package strategy, desktop environment, language/runtime choices, core service architecture, UI framework, data/storage architecture, security boundaries, test framework, CI strategy, ISO/build architecture.
- **Minimal bootable PURSUE base (Phase 1B)** — reproducible development/build environment and a bootable minimal PURSUE image.
- **Core runtime (Phase 1C)** — core services, configuration, logging, secure storage foundation, IPC/service boundaries, update/package foundations, test harness.
- **Case/evidence foundation (Phase 1D)** — case model, evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata.
- **Terminal foundation (Phase 1E)** — flagship; dedicated phase.
- **Browser + Tor foundation (Phase 1F)** — dedicated major phase; Tor integration in detail is to be handled together with the terminal.
- **Plugin foundation** — strong, extensible; roughly 5–7 useful tools per use case initially.
- **Optional local AI assistance** — local models only; no external AI APIs; fast / deep-thinking / coding model categories; non-AI mode.
- **Professional reporting basics** — ZIP and JSON export; provenance and integrity preserved.

## Explicitly Out of Scope (V1)

From the handoff's "WHAT NOT TO DO NEXT" and locked phase boundaries:

- Full Investigation Terminal and full Investigation Browser implementations (foundations only in V1).
- Tor integration as a completed feature (foundation/research only).
- Large OSINT tool collection — defaults are curated (5–7 per use case), not exhaustive.
- Advanced DFIR tooling.
- Graph UI and reporting UI.
- Plugin marketplace / public plugin distribution (V2).
- Advanced AI configuration and advanced customization (later updates).
- External AI APIs (V1 hard non-goal).
- Production release and final ISO (no stable release exists; do not create a fake ISO).
- Anonymity/security guarantees — none are claimed.

## Deferred to V2+ (from handoff)

- Stronger plugin system.
- Additional official modules.
- Downloadable advanced modules.
- Future plugins posted publicly / on GitHub.
