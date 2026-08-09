# PURSUE OS — System Architecture

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative architecture boundaries. Implementation detail is Category B.

## Architectural Shape (from handoff)

- **Investigation interface layer** — two flagship interfaces: the Investigation Terminal and the Investigation Browser. Together they form the major investigation interface layer.
- **Investigation platform layer** — case system, evidence handling and storage, provenance, investigation graphs and timelines, reporting and export, secure research.
- **Modular tool layer** — OSINT/SOCMINT/GEOINT/CTI/DFIR/infrastructure/media tooling; curated defaults (~5–7 per use case); existing tools integrated where appropriate; official tools where necessary.
- **Plugin layer** — strong V1 foundation designed so the V2 plugin system (official modules, downloadable advanced modules, public plugins) does not require rebuilding.
- **AI layer (optional)** — local models only in V1; assistant-only role; non-AI mode; strict AI boundaries.
- **Security layer** — privilege separation, secure storage, evidence integrity, network boundaries, Tor isolation, plugin isolation, AI boundaries, update security, package integrity, case isolation, secure defaults.

## Foundation Phases (from handoff)

1. **1A** Technical architecture lock (base, build system, package strategy, desktop environment, languages, core services, UI framework, data/storage, security boundaries, test framework, CI, ISO/build architecture).
2. **1B** Minimal bootable PURSUE base — reproducible development/build environment, eventually a bootable minimal image.
3. **1C** Core runtime — core services, configuration, logging, secure storage foundation, IPC/service boundaries, update/package foundations, test harness.
4. **1D** Case/evidence foundation — case model, evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata.
5. **1E** Terminal foundation (flagship).
6. **1F** Browser + Tor foundation (dedicated major phase; Tor in detail together with the terminal).

## Implementation-Defined Detail

Exact services, IPC transport, storage technology, UI framework, and deployment units are implementation-defined (Category B) and are locked during Phase 1A–1C. See DECISION_RECORD Part B for decisions made so far.
