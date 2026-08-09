# PURSUE OS — Service Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A/B — service *areas* are handoff-locked; service *decomposition* is implementation-defined.

## Service Areas (locked by handoff — Phase 1C "core runtime")

- Core services
- Configuration
- Logging
- Secure storage foundation
- IPC / service boundaries
- Update / package foundations
- Test harness

## Interface Services (locked flagships)

- Investigation Terminal service (flagship; Phase 1E).
- Investigation Browser service (flagship; Phase 1F with Tor foundation).

## Platform Services (locked areas, Phase 1D)

- Case service (case model, evidence, sources, notes, relationships, timelines, findings, reports, exports).
- Evidence service (collection, storage, provenance, integrity).
- Provenance service (traceability of evidence and investigator actions).
- Reporting/export service (ZIP, JSON; preserves provenance and integrity).

## Implementation-Defined

The concrete decomposition — daemons, IPC transport, sandboxing units, and which services are privileged vs. unprivileged — is implementation-defined (Category B) and locked during Phases 1A–1C. See DECISION_RECORD Part B for the technical decisions made so far (Rust workspace; evidence foundation crates).
