# PURSUE OS — Provenance Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative provenance requirements.

## Locked Requirements

- **Provenance is foundational.** Every piece of evidence must remain traceable to its source and to investigator actions.
- Evidence integrity and provenance are foundational properties of the system (see SECURITY.md: "Evidence must remain traceable to its source and investigator actions").
- Sources: provenance connects information to its source; AI must not silently mutate source information.
- Case model: provenance is one of the things every case supports.

## Principles

- Evidence is always the source of truth; provenance is how that truth is verifiable.
- AI must never alter evidence or silently modify sources, so provenance can never be attributed to AI actions beyond recorded assistance.
- Tampering with provenance records must be detectable (evidence integrity).

## Implementation-Defined

The provenance record format is implementation-defined (Category B). The evidence foundation implements an append-only, hash-chained audit/event structure that records actions against evidence and sources, with tamper detection (see DECISION_RECORD B3).
